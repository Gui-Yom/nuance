use std::io;
use std::io::Write;
use std::thread::spawn;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen, SetTitle,
};
use crossterm::{event, execute};
use log::{debug, info, LevelFilter};
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, BorderType, Borders};
use tui::Terminal;
use tui_logger::{init_logger, set_default_level, set_log_file, TuiLoggerWidget};
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use crate::app::Command;
use crate::input::{InputBox, InputBoxState};

mod app;
mod input;
mod shader_loader;

fn main() -> Result<()> {
    // Setup the tui
    //crossterm::terminal::enable_raw_mode();

    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        SetTitle("Shadertoy")
    )?;
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    init_logger(LevelFilter::Debug)?;
    set_default_level(LevelFilter::Debug);
    set_log_file("shadertoy.log");

    let event_loop = EventLoop::with_user_event();
    let ev_sender = event_loop.create_proxy();

    let cli_thread = spawn(move || -> Result<()> {
        let mut input_box_state = InputBoxState::default();

        loop {
            if event::poll(Duration::from_millis(500))? {
                match event::read()? {
                    Event::Key(KeyEvent { code, modifiers }) => match code {
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
                            let mut command = command.split(" ");
                            match command.next() {
                                Some("load") => {
                                    let file =
                                        command.next().context("You need to specify a shader")?;
                                    debug!("load {}", file);
                                    ev_sender.send_event(Command::Load(file.to_string()))?;
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
                    },
                    _ => {}
                }
            }
            terminal.draw(|frame| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(90), Constraint::Max(2)])
                    .split(frame.size());

                let widget = TuiLoggerWidget::default().block(
                    Block::default()
                        .borders(Borders::BOTTOM)
                        .border_type(BorderType::Plain),
                );
                frame.render_widget(widget, chunks[0]);

                let input_box = InputBox {};
                frame.render_stateful_widget(input_box, chunks[1], &mut input_box_state);
            })?;
        }
        ev_sender.send_event(Command::Close)?;
        Ok(())
    });

    // Create the window
    let builder = WindowBuilder::new()
        .with_title("Shadertoy")
        .with_inner_size(LogicalSize::new(800, 600))
        .with_resizable(false)
        .with_visible(true);
    let window = builder.build(&event_loop)?;

    // Going async !
    futures_executor::block_on(app::run(window, event_loop))?;

    cli_thread.join().expect("Can't join thread ?");

    disable_raw_mode()?;
    execute!(io::stdout(), Clear(ClearType::All), LeaveAlternateScreen)?;
    Ok(())
}
