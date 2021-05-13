//! Extract information from glsl source and transpiles it to valid glsl source code.

use core::panic;
use std::ops::Deref;

use anyhow::{anyhow, Result};
use glsl_lang::ast::{PreprocessorDefine, TypeSpecifierNonArray};
use glsl_lang::{
    ast::{
        Block, Expr, Identifier, IdentifierData, LayoutQualifier, LayoutQualifierSpec, SmolStr,
        StructFieldSpecifier, TranslationUnit, TypeQualifierSpec,
    },
    parse::{Parsable, ParseOptions},
    transpiler::glsl::FormattingState,
    visitor::{HostMut, Visit, VisitorMut},
};
use log::error;

use crate::shader::{ShaderMetadata, Slider};
use crate::types::Vec3f;

impl VisitorMut for ShaderMetadata {
    fn visit_block(&mut self, block: &mut Block) -> Visit {
        if let Some(TypeQualifierSpec::Layout(layout)) = block.qualifier.qualifiers.first() {
            if let Some(LayoutQualifierSpec::Identifier(id, _)) = layout.ids.first() {
                if id.content.0 == "params" {
                    // We got the block we searched for
                    for field in block.fields.iter_mut() {
                        if let Ok(slider) = create_slider_from_field(field) {
                            self.sliders.push(slider);
                            convert_field(field);
                        } else {
                            panic!("Invalid field");
                        }
                    }
                    convert_block(block);
                }
            }
        }
        Visit::Parent
    }

    fn visit_preprocessor_define(&mut self, define: &mut PreprocessorDefine) -> Visit {
        if let PreprocessorDefine::ObjectLike { ident, .. } = define {
            if ident.content.0.as_str() == "NUANCE_STILL_IMAGE" {
                self.still_image = true;
            }
        }
        Visit::Parent
    }

    fn visit_expr(&mut self, expr: &mut Expr) -> Visit {
        if let Expr::Dot(expr2, ident1) = expr {
            if let Expr::Variable(ident0) = expr2.as_ref() {
                let slider_name = ident0.content.0.as_str();
                for slider in self.sliders.iter() {
                    match slider {
                        Slider::Float { name, min, max, .. } => {
                            if name == slider_name {
                                *expr = Expr::FloatConst(match ident1.content.0.as_str() {
                                    "max" => *max,
                                    "min" => *min,
                                    _ => panic!("No such field exist !"),
                                });
                                return Visit::Parent;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Visit::Children
    }
}

pub fn create_slider_from_field(field: &StructFieldSpecifier) -> Result<Slider> {
    let name = field
        .identifiers
        .first()
        .unwrap()
        .ident
        .content
        .0
        .to_string();

    match field.ty.ty {
        // To Slider::Float
        TypeSpecifierNonArray::Float => {
            let mut min = 0.0;
            let mut max = 1.0;
            let mut init = 0.0;
            if let TypeQualifierSpec::Layout(LayoutQualifier { ids }) = field
                .qualifier
                .as_ref()
                .unwrap()
                .qualifiers
                .first()
                .unwrap()
            {
                for qualifier in ids.iter() {
                    if let LayoutQualifierSpec::Identifier(id, param) = qualifier {
                        match id.0.as_str() {
                            "min" => {
                                min = param.as_ref().unwrap().deref().coerce_const();
                            }
                            "max" => {
                                max = param.as_ref().unwrap().deref().coerce_const();
                            }
                            "init" => {
                                init = param.as_ref().unwrap().deref().coerce_const();
                            }
                            other => {
                                error!("Wrong slider setting : {}", other)
                            }
                        }
                    }
                }
            }
            return Ok(Slider::Float {
                name,
                min,
                max,
                value: init,
            });
        }
        // To Slider::Color if color layout qualifier is set
        TypeSpecifierNonArray::Vec3 => {
            if let TypeQualifierSpec::Layout(LayoutQualifier { ids }) = field
                .qualifier
                .as_ref()
                .unwrap()
                .qualifiers
                .first()
                .unwrap()
            {
                if let Some(LayoutQualifierSpec::Identifier(ident, _)) = ids.first() {
                    if ident.content.0.as_str() == "color" {
                        return Ok(Slider::Color {
                            name,
                            value: Vec3f::zero(),
                        });
                    }
                }
            }
        }
        _ => {}
    }
    Err(anyhow!("Invalid field in params block"))
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

pub fn extract(source: &str) -> Result<(ShaderMetadata, String)> {
    let mut metadata = ShaderMetadata::default();

    // The AST
    let (mut ast, _ctx) = TranslationUnit::parse_with_options(
        source,
        &ParseOptions {
            target_vulkan: true,
            source_id: 0,
            allow_rs_ident: false,
        }
        .build(),
    )?;

    // Extract some ast juice
    ast.visit_mut(&mut metadata);

    let mut transpiled = String::new();
    glsl_lang::transpiler::glsl::show_translation_unit(
        &mut transpiled,
        &ast,
        FormattingState::default(),
    )?;
    Ok((metadata, transpiled))
}

trait CoerceConst<T> {
    fn coerce_const(&self) -> T;
}

impl CoerceConst<f32> for Expr {
    fn coerce_const(&self) -> f32 {
        match self {
            Expr::IntConst(value) => *value as f32,
            Expr::UIntConst(value) => *value as f32,
            Expr::FloatConst(value) => *value,
            Expr::DoubleConst(value) => *value as f32,
            _ => {
                panic!("Not a number constant")
            }
        }
    }
}

impl CoerceConst<f64> for Expr {
    fn coerce_const(&self) -> f64 {
        match self {
            Expr::IntConst(value) => *value as f64,
            Expr::UIntConst(value) => *value as f64,
            Expr::FloatConst(value) => *value as f64,
            Expr::DoubleConst(value) => *value,
            _ => {
                panic!("Not a number constant")
            }
        }
    }
}

impl CoerceConst<i32> for Expr {
    fn coerce_const(&self) -> i32 {
        match self {
            Expr::IntConst(value) => *value,
            Expr::UIntConst(value) => *value as i32,
            Expr::FloatConst(value) => *value as i32,
            Expr::DoubleConst(value) => *value as i32,
            _ => {
                panic!("Not a number constant")
            }
        }
    }
}

impl CoerceConst<u32> for Expr {
    fn coerce_const(&self) -> u32 {
        match self {
            Expr::IntConst(value) => *value as u32,
            Expr::UIntConst(value) => *value,
            Expr::FloatConst(value) => *value as u32,
            Expr::DoubleConst(value) => *value as u32,
            _ => {
                panic!("Not a number constant")
            }
        }
    }
}
