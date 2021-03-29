//! Extract information from glsl source and transpiles it to valid glsl source code.

use std::ops::{Deref, RangeBounds, RangeInclusive};

use glsl_lang::{
    ast::{
        Block, Expr, Identifier, IdentifierData, LayoutQualifier, LayoutQualifierSpec, SmolStr,
        StructFieldSpecifier, TranslationUnit, TypeQualifierSpec,
    },
    parse::{Parsable, ParseOptions},
    transpiler::glsl::FormattingState,
    visitor::{HostMut, Visit, VisitorMut},
};
use log::debug;

pub struct Slider {
    pub name: String,
    pub range: RangeInclusive<f32>,
    pub step: f32,
    pub value: f32,
}

/// Traverses the ast and extract useful data while converting the ast to valid glsl source
struct Extractor {
    sliders: Vec<Slider>,
}

impl VisitorMut for Extractor {
    fn visit_block(&mut self, block: &mut Block) -> Visit {
        if let Some(TypeQualifierSpec::Layout(layout)) = block.qualifier.qualifiers.first() {
            if let Some(LayoutQualifierSpec::Identifier(id, _)) = layout.ids.first() {
                if id.content.0 == "params" {
                    // We got the block we searched for
                    for field in block.fields.iter_mut() {
                        self.sliders.push(create_slider_from_field(field));
                        // FIXME field might not need transpiling if we allow default sliders
                        convert_field(field);
                    }
                    convert_block(block);
                }
            }
        }
        Visit::Parent
    }

    fn visit_expr(&mut self, expr: &mut Expr) -> Visit {
        if let Expr::Dot(expr2, ident1) = expr {
            if let Expr::Variable(ident0) = expr2.as_ref() {
                let name = ident0.content.0.as_str();
                if let Some(slider) = self.sliders.iter().find(|it| it.name == name) {
                    *expr = Expr::FloatConst(match ident1.content.0.as_str() {
                        "max" => {
                            let bound = slider.range.end_bound();
                            match bound {
                                std::ops::Bound::Included(value) => *value,
                                _ => panic!("Wrong range !"),
                            }
                        }
                        "min" => {
                            let bound = slider.range.start_bound();
                            match bound {
                                std::ops::Bound::Included(value) => *value,
                                _ => panic!("Wrong range !"),
                            }
                        }
                        _ => panic!("No such field exist !"),
                    })
                } else {
                    debug!("no slider with name '{}' exist", name);
                }
                return Visit::Parent;
            }
        }
        Visit::Children
    }
}

// TODO different sliders for different field types
pub fn create_slider_from_field(field: &StructFieldSpecifier) -> Slider {
    let name = field
        .identifiers
        .first()
        .unwrap()
        .ident
        .content
        .0
        .to_string();

    // TODO different sliders and params based on field type

    let (range, step, value) = if let TypeQualifierSpec::Layout(LayoutQualifier { ids }) = field
        .qualifier
        .as_ref()
        .unwrap()
        .qualifiers
        .first()
        .unwrap()
    {
        let mut min: f32 = 0.0;
        let mut max: f32 = 100.0;
        let mut step: f32 = 1.0;
        let mut init: f32 = 0.0;
        for qualifier in ids.iter() {
            if let LayoutQualifierSpec::Identifier(id, param) = qualifier {
                match id.0.as_str() {
                    "min" => match param.as_ref().unwrap().deref() {
                        Expr::FloatConst(value) => min = *value,
                        _ => {}
                    },
                    "max" => match param.as_ref().unwrap().deref() {
                        Expr::FloatConst(value) => max = *value,
                        _ => {}
                    },
                    "step" => match param.as_ref().unwrap().deref() {
                        Expr::FloatConst(value) => step = *value,
                        _ => {}
                    },
                    "init" => match param.as_ref().unwrap().deref() {
                        Expr::FloatConst(value) => init = *value,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        (min..=max, step, init)
    } else {
        (0.0..=100.0, 1.0, 0.0)
    };
    Slider {
        name,
        range,
        step,
        value,
    }
}

/// Replace the layout(params) with a predefined layout(set=?, binding=?)
pub fn convert_block(block: &mut Block) {
    block.qualifier.qualifiers[0] = TypeQualifierSpec::Layout(LayoutQualifier {
        ids: vec![
            LayoutQualifierSpec::Identifier(
                Identifier {
                    content: IdentifierData(SmolStr::new("set")),
                    span: None,
                },
                Some(Box::new(Expr::IntConst(0))),
            ),
            LayoutQualifierSpec::Identifier(
                Identifier {
                    content: IdentifierData(SmolStr::new("binding")),
                    span: None,
                },
                Some(Box::new(Expr::IntConst(0))),
            ),
        ],
    });
}

/// Replace the layout(min=?, max=?) with nothing
pub fn convert_field(field: &mut StructFieldSpecifier) {
    field.qualifier = None;
}

pub fn extract(source: &str) -> Option<(Vec<Slider>, String)> {
    let mut extractor = Extractor {
        sliders: Vec::new(),
    };

    // The AST
    let (mut ast, _ctx) = TranslationUnit::parse_with_options(
        source,
        &ParseOptions {
            target_vulkan: true,
            source_id: 0,
            allow_rs_ident: false,
        }
        .build(),
    )
    .expect("Invalid GLSL source.");

    // Extract some ast juice
    ast.visit_mut(&mut extractor);

    let mut transpiled = String::new();
    glsl_lang::transpiler::glsl::show_translation_unit(
        &mut transpiled,
        &ast,
        FormattingState::default(),
    )
    .expect("Can't transpile ast");
    Some((extractor.sliders, transpiled))
}
