use std::io;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::spawn;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen, SetTitle,
};
use crossterm::{event, execute};
use log::{debug, LevelFilter};
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::Terminal;
use tui_logger::{init_logger, set_default_level, set_log_file, TuiLoggerWidget};
use wgpu::PowerPreference;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use nuance::{Command, Shadyboi};

use crate::input::{InputBox, InputBoxState};

mod input;

static should_exit: AtomicBool = AtomicBool::new(false);

fn main() -> Result<()> {
    // Setup the tui
    //crossterm::terminal::enable_raw_mode();

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        SetTitle("Nuance")
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    //let level = LevelFilter::Debug;
    init_logger(LevelFilter::Info)?;
    set_default_level(LevelFilter::Info);
    set_log_file("nuance.log")?;

    let event_loop = EventLoop::with_user_event();
    let ev_sender = event_loop.create_proxy();

    let cli_thread = spawn(move || -> Result<()> {
        let mut input_box_state = InputBoxState::default();

        loop {
            if should_exit.load(Ordering::Relaxed) {
                return Ok(());
            }
            if event::poll(Duration::from_millis(1000))? {
                if let Event::Key(KeyEvent { code, modifiers }) = event::read()? {
                    match code {
                        KeyCode::Esc => {
                            break;
                        }
                        KeyCode::Backspace | KeyCode::Delete | KeyCode::Left | KeyCode::Right => {
                            input_box_state.process_event(code);
                        }
                        KeyCode::Char(c) => {
                            if c == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                                break;
                            } else {
                                input_box_state.process_event(code);
                            }
                        }
                        KeyCode::Enter => {
                            let command = input_box_state.text();
                            debug!("{}", command);
                            input_box_state.clear();

                            // Here we go
                            let mut command = command.split(' ');
                            match command.next() {
                                Some("load") => {
                                    let file =
                                        command.next().context("You need to specify a shader")?;
                                    debug!("load {}", file);
                                    ev_sender.send_event(Command::Load(file.to_string()))?;
                                }
                                Some("reload") => {
                                    debug!("reload");
                                    ev_sender.send_event(Command::Reload)?;
                                }
                                Some("watch") => {
                                    let file =
                                        command.next().context("You need to specify a shader")?;
                                    debug!("watch {}", file);
                                    ev_sender.send_event(Command::Load(file.to_string()))?;
                                    ev_sender.send_event(Command::Watch(file.to_string()))?;
                                }
                                Some("unwatch") => {
                                    debug!("unwatch");
                                    ev_sender.send_event(Command::Unwatch)?;
                                }
                                Some("framerate") => {
                                    let fps: i16 = command
                                        .next()
                                        .context("You need to specify a shader")?
                                        .parse()?;
                                    debug!("framerate {}", fps);
                                    ev_sender.send_event(Command::TargetFps(fps))?;
                                }
                                Some("restart") => {
                                    debug!("restart");
                                    ev_sender.send_event(Command::Restart)?;
                                }
                                Some("exit") => {
                                    break;
                                }
                                _ => {}
                            }
                        }
                        _ => {
                            debug!("{:?}", code);
                        }
                    }
                }
            }
            terminal.draw(|frame| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(90), Constraint::Max(3)])
                    .split(frame.size());

                let widget = TuiLoggerWidget::default();
                frame.render_widget(widget, chunks[0]);

                let input_box = InputBox;
                frame.render_stateful_widget(input_box, chunks[1], &mut input_box_state);
            })?;
        }
        ev_sender.send_event(Command::Exit)?;
        Ok(())
    });

    // Create the window
    let builder = WindowBuilder::new()
        .with_title("Shadertoy")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_resizable(false)
        .with_visible(true);
    let window = builder.build(&event_loop)?;

    // GPU power preference
    let args: Vec<String> = std::env::args().collect();
    let power_preference = if args.contains(&String::from("high")) {
        PowerPreference::HighPerformance
    } else {
        PowerPreference::LowPower
    };

    // Going async !
    let app = futures_executor::block_on(Shadyboi::init(window, power_preference))?;
    futures_executor::block_on(app.run(event_loop))?;

    should_exit.store(true, Ordering::Relaxed);

    cli_thread.join().unwrap()?;

    execute!(io::stdout(), Clear(ClearType::All), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
