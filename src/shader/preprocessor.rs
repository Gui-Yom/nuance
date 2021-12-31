//! Extract information from glsl source and transpiles it to valid glsl source code.

use core::panic;
use std::borrow::Borrow;

use anyhow::{anyhow, Result};
use glsl_lang::ast::{
    BlockData, ExprData, FunIdentifierData, IdentifierData, LayoutQualifierData,
    LayoutQualifierSpecData, Node, PreprocessorDefineData, SmolStr, StructFieldSpecifierData,
    TranslationUnit, TypeQualifierData, TypeQualifierSpecData, TypeSpecifierData,
    TypeSpecifierNonArrayData,
};
use glsl_lang::parse::{DefaultLexer, Parse, ParseBuilder, ParseContext, ParseOptions};
use glsl_lang::transpiler::glsl::{show_translation_unit, FormattingState};
use glsl_lang::visitor::{HostMut, Visit, VisitorMut};
use lang_util::FileId;
use log::{debug, error};
use mint::{Vector2, Vector3};

use crate::{ShaderMetadata, Slider};

impl VisitorMut for ShaderMetadata {
    fn visit_block(&mut self, block: &mut Node<BlockData>) -> Visit {
        let block = &mut block.content;
        // Find a params block which is a GLSL uniform block with the layout(params) qualifier
        if let Some(TypeQualifierSpecData::Layout(layout)) = block
            .qualifier
            .content
            .qualifiers
            .first()
            .map(|x| &x.content)
        {
            if let Some(LayoutQualifierSpecData::Identifier(id, _)) =
                layout.content.ids.first().map(|x| &x.content)
            {
                if id.content.0 == "params" {
                    // We got the block we searched for
                    for field in block.fields.iter_mut() {
                        if let Ok(slider) = create_slider_from_field(&field.content) {
                            self.sliders.push(slider);
                            // Remove the layout(min=?, max=?) annotation on params block fields
                            field.content.qualifier = None;
                        } else {
                            panic!("Invalid field");
                        }
                    }
                    convert_params_block(block);
                }
            }
        }
        Visit::Parent
    }

    fn visit_preprocessor_define(&mut self, define: &mut Node<PreprocessorDefineData>) -> Visit {
        if let PreprocessorDefineData::ObjectLike { ident, .. } = &define.content {
            if ident.content.0.as_str() == "NUANCE_STILL_IMAGE" {
                self.still_image = true;
            }
        }
        Visit::Parent
    }

    fn visit_expr(&mut self, expr: &mut Node<ExprData>) -> Visit {
        if let ExprData::Dot(expr2, ident1) = &mut expr.content {
            if let ExprData::Variable(ident0) = &expr2.as_ref().content {
                let slider_name = ident0.content.0.as_str();
                for slider in self.sliders.iter() {
                    match slider {
                        Slider::Float {
                            name,
                            min,
                            max,
                            default,
                            ..
                        } => {
                            if name == slider_name {
                                expr.content =
                                    ExprData::FloatConst(match ident1.content.0.as_str() {
                                        "max" => *max,
                                        "min" => *min,
                                        "init" => *default,
                                        // No . accessors on a float value
                                        other => {
                                            panic!("No such property '{}' on float param", other)
                                        }
                                    });
                                return Visit::Parent;
                            }
                        }
                        Slider::Vec2 { name, default, .. } => {
                            if name == slider_name {
                                match ident1.content.0.as_str() {
                                    "init" => {
                                        expr.content = ExprData::FunCall(
                                            FunIdentifierData::TypeSpecifier(Box::new(
                                                TypeSpecifierData {
                                                    ty: TypeSpecifierNonArrayData::Vec2.into(),
                                                    array_specifier: None,
                                                }
                                                .into(),
                                            ))
                                            .into(),
                                            default
                                                .as_ref()
                                                .iter()
                                                .map(|it| ExprData::FloatConst(*it).into())
                                                .collect(),
                                        );
                                    }
                                    _ => {}
                                }
                                return Visit::Parent;
                            }
                        }
                        Slider::Vec3 { name, default, .. } => {
                            if name == slider_name {
                                match ident1.content.0.as_str() {
                                    "init" => {
                                        expr.content = ExprData::FunCall(
                                            FunIdentifierData::TypeSpecifier(Box::new(
                                                TypeSpecifierData {
                                                    ty: TypeSpecifierNonArrayData::Vec3.into(),
                                                    array_specifier: None,
                                                }
                                                .into(),
                                            ))
                                            .into(),
                                            default
                                                .as_ref()
                                                .iter()
                                                .map(|it| ExprData::FloatConst(*it).into())
                                                .collect(),
                                        );
                                    }
                                    _ => {}
                                }
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

fn process_layout_qualifier_on_field(
    field: &StructFieldSpecifierData,
    mut consumer: impl FnMut(&str, &Node<ExprData>),
) {
    // Does the field has any qualifiers ?
    if let Some(TypeQualifierData { qualifiers }) = field.qualifier.as_ref().map(|x| &x.content) {
        qualifiers
            .iter()
            .map(|x| &x.content)
            .filter_map(|x| match x {
                // Extract key value pairs from layout qualifiers
                TypeQualifierSpecData::Layout(Node {
                    content: LayoutQualifierData { ids },
                    span: _,
                }) => Some(ids),
                _ => None,
            })
            .flatten()
            .for_each(|pair| {
                if let LayoutQualifierSpecData::Identifier(
                    Node {
                        content: IdentifierData(key),
                        span: _,
                    },
                    Some(value),
                ) = &pair.content
                {
                    consumer(key.as_str(), value.as_ref());
                }
            });
    }
}

pub fn create_slider_from_field(field: &StructFieldSpecifierData) -> Result<Slider> {
    let name = field
        .identifiers
        .first()
        .unwrap()
        .content
        .ident
        .content
        .0
        .to_string();

    //debug!("{:#?}", field);

    match field.ty.content.ty.content {
        // To Slider::Float
        TypeSpecifierNonArrayData::Float => {
            let mut min = 0.0;
            let mut max = 1.0;
            let mut init = 0.0;

            process_layout_qualifier_on_field(field, |key, value| match key {
                "min" => {
                    min = value.coerce_const();
                }
                "max" => {
                    max = value.coerce_const();
                }
                "init" => {
                    init = value.coerce_const();
                }
                other => {
                    error!("Wrong slider setting : {}", other)
                }
            });
            return Ok(Slider::Float {
                name,
                min,
                max,
                value: init,
                default: init,
            });
        }
        // To Slider::Uint
        TypeSpecifierNonArrayData::UInt => {
            let mut min = 0;
            let mut max = 100;
            let mut init = 0;

            process_layout_qualifier_on_field(field, |key, value| match key {
                "min" => min = value.coerce_const(),
                "max" => {
                    max = value.coerce_const();
                }
                "init" => {
                    init = value.coerce_const();
                }
                other => {
                    error!("Wrong slider setting : {}", other)
                }
            });

            return Ok(Slider::Uint {
                name,
                min,
                max,
                value: init,
                default: init,
            });
        }
        TypeSpecifierNonArrayData::Vec2 => {
            let mut init: Vector2<f32> = Vector2::from([0.0, 0.0]);

            process_layout_qualifier_on_field(field, |key, value| match key {
                "init" => {
                    if let ExprData::FunCall(
                        Node {
                            content: FunIdentifierData::TypeSpecifier(ty_spec),
                            span: _,
                        },
                        params,
                    ) = &value.content
                    {
                        if ty_spec.content.ty.content == TypeSpecifierNonArrayData::Vec2
                            && params.len() == 2
                        {
                            init =
                                Vector2::from([params[0].coerce_const(), params[1].coerce_const()]);
                        } else {
                            error!("Invalid initializer ! Only constant arguments are accepted.");
                        }
                    }
                }
                other => {
                    error!("Unsupported setting : {}", other)
                }
            });

            return Ok(Slider::Vec2 {
                name,
                value: init,
                default: init,
            });
        }
        // To Slider::Color if color layout qualifier is set
        TypeSpecifierNonArrayData::Vec3 => {
            let mut init: Vector3<f32> = Vector3::from([0.0, 0.0, 0.0]);
            let mut color = false;

            process_layout_qualifier_on_field(field, |key, value| match key {
                "color" => {
                    color = true;
                }
                "init" => {
                    if let ExprData::FunCall(
                        Node {
                            content: FunIdentifierData::TypeSpecifier(ty_spec),
                            span: _,
                        },
                        params,
                    ) = &value.content
                    {
                        if ty_spec.content.ty.content == TypeSpecifierNonArrayData::Vec3
                            && params.len() == 3
                        {
                            init = Vector3::from([
                                params[0].coerce_const(),
                                params[1].coerce_const(),
                                params[2].coerce_const(),
                            ]);
                        } else {
                            error!("Invalid initializer !");
                        }
                    }
                }
                other => {
                    error!("Unsupported setting : {}", other)
                }
            });
            return Ok(if color {
                Slider::Color {
                    name,
                    value: init,
                    default: init,
                }
            } else {
                Slider::Vec3 {
                    name,
                    value: init,
                    default: init,
                }
            });
        }
        TypeSpecifierNonArrayData::Bool => {
            let mut init = 0;

            process_layout_qualifier_on_field(field, |key, value| match key {
                "init" => match value.content {
                    ExprData::BoolConst(value) => {
                        init = if value { 1 } else { 0 };
                    }
                    _ => {
                        error!("Expected boolean value");
                    }
                },
                other => {
                    error!("Wrong slider setting : {}", other);
                }
            });

            return Ok(Slider::Bool {
                name,
                value: init,
                default: init,
            });
        }
        _ => {}
    }
    Err(anyhow!("Invalid field in params block"))
}

/// Replace the layout(params) with a predefined layout(set=?, binding=?)
pub fn convert_params_block(block: &mut BlockData) {
    // I could have used glsl-lang-quote instead of creating the ast by hand
    block.qualifier.content.qualifiers[0] = TypeQualifierSpecData::Layout(
        LayoutQualifierData {
            ids: vec![
                LayoutQualifierSpecData::Identifier(
                    IdentifierData(SmolStr::new("std140")).into(),
                    None,
                )
                .into(),
                LayoutQualifierSpecData::Identifier(
                    IdentifierData(SmolStr::new("set")).into(),
                    Some(Box::new(ExprData::IntConst(1).into())),
                )
                .into(),
                LayoutQualifierSpecData::Identifier(
                    IdentifierData(SmolStr::new("binding")).into(),
                    Some(Box::new(ExprData::IntConst(0).into())),
                )
                .into(),
            ],
        }
        .into(),
    )
    .into();
}

pub fn extract(source: &str) -> Result<(ShaderMetadata, String)> {
    let mut metadata = ShaderMetadata::default();

    let (mut ast, _, _) = ParseBuilder::<DefaultLexer, TranslationUnit>::new(source)
        .opts(&ParseOptions {
            default_version: 460,
            target_vulkan: true,
            source_id: FileId::new(0),
            allow_rs_ident: false,
        })
        .context(&ParseContext::new_with_comments())
        .parse()?;

    // Extract some ast juice
    ast.visit_mut(&mut metadata);

    let mut transpiled = String::new();
    show_translation_unit(&mut transpiled, &ast, FormattingState::default())?;
    debug!("{}", &transpiled);
    Ok((metadata, transpiled))
}

trait CoerceConst<T> {
    fn coerce_const(&self) -> T;
}

macro_rules! coerceconst_impl {
    ($ty:ty) => {
        impl<T> CoerceConst<$ty> for T
        where
            T: Borrow<ExprData>,
        {
            fn coerce_const(&self) -> $ty {
                match self.borrow() {
                    ExprData::IntConst(value) => *value as $ty,
                    ExprData::UIntConst(value) => *value as $ty,
                    ExprData::FloatConst(value) => *value as $ty,
                    ExprData::DoubleConst(value) => *value as $ty,
                    ExprData::BoolConst(value) => {
                        if *value {
                            1 as $ty
                        } else {
                            0 as $ty
                        }
                    }
                    _ => {
                        panic!("Anything other than a constant is not supported")
                    }
                }
            }
        }
    };
}

coerceconst_impl!(f32);
coerceconst_impl!(f64);
coerceconst_impl!(u32);
coerceconst_impl!(i32);
