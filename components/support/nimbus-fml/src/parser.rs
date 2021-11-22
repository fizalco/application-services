/* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::{collections::HashMap, convert::TryFrom, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    error::FMLError,
    intermediate_representation::{
        EnumDef, FeatureDef, FeatureManifest, ObjectDef, PropDef, TypeRef, VariantDef,
    },
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct EnumVariantBody {
    description: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct EnumBody {
    description: String,
    variants: HashMap<String, EnumVariantBody>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ObjectFieldBody {
    description: String,
    #[serde(default)]
    required: bool,
    #[serde(rename = "type")]
    variable_type: String,
    default: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ObjectBody {
    description: String,
    failable: Option<bool>,
    fields: HashMap<String, ObjectFieldBody>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct Types {
    enums: HashMap<String, EnumBody>,
    objects: HashMap<String, ObjectBody>,
}

impl TryFrom<String> for TypeRef {
    fn try_from(val: String) -> Result<TypeRef, FMLError> {
        let (type_ref, type_name) = parse_typeref_string(val)?;

        return match type_ref.as_str() {
            "String" => Ok(TypeRef::String),
            "Int" => Ok(TypeRef::Int),
            "Boolean" => Ok(TypeRef::Boolean),
            "BundleText" => Ok(TypeRef::BundleText(type_name.unwrap())),
            "BundleImage" => Ok(TypeRef::BundleImage(type_name.unwrap())),
            "Enum" => Ok(TypeRef::Enum(type_name.unwrap())),
            "Object" => Ok(TypeRef::Object(type_name.unwrap())),
            "List" => Ok(TypeRef::List(Box::new(TypeRef::try_from(
                type_name.unwrap(),
            )?))),
            "Option" => Ok(TypeRef::Option(Box::new(TypeRef::try_from(
                type_name.unwrap(),
            )?))),
            "Map" => {
                // Maps take a little extra massaging to get the key and value types
                let type_name = type_name.unwrap();
                let mut map_type_info_iter = type_name.split(',');

                let key_type = map_type_info_iter.next().unwrap().to_string();
                let value_type = map_type_info_iter.next().unwrap().trim().to_string();

                if key_type.starts_with("Enum") {
                    Ok(TypeRef::EnumMap(
                        Box::new(TypeRef::try_from(key_type)?),
                        Box::new(TypeRef::try_from(value_type)?),
                    ))
                } else if key_type.eq("String") {
                    Ok(TypeRef::StringMap(Box::new(TypeRef::try_from(value_type)?)))
                } else {
                    Err(FMLError::TypeParsingError(format!(
                        "{} is not a recognized FML Map key type",
                        type_ref
                    )))
                }
            }
            _ => Err(FMLError::TypeParsingError(format!(
                "{} is not a recognized FML type",
                type_ref
            ))),
        };
    }

    type Error = FMLError;
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct FeatureVariableBody {
    description: String,
    #[serde(rename = "type")]
    variable_type: String,
    default: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct FeatureVariable {
    name: String,
    body: FeatureVariableBody,
}

impl TryFrom<FeatureVariable> for PropDef {
    fn try_from(fv: FeatureVariable) -> Result<PropDef, FMLError> {
        Ok(PropDef {
            name: fv.name,
            doc: fv.body.description,
            typ: TypeRef::try_from(fv.body.variable_type)?,
            default: json!(&fv.body.default),
        })
    }

    type Error = FMLError;
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct FeatureBody {
    description: String,
    variables: HashMap<String, FeatureVariableBody>,
    default: Option<serde_json::Value>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ManifestFrontEnd {
    types: Types,
    features: HashMap<String, FeatureBody>,
    channels: Vec<String>,
}

fn parse_typeref_string(input: String) -> Result<(String, Option<String>), FMLError> {
    // Split the string into the TypeRef and the name
    let mut object_type_iter = input.split(&['<', '>'][..]);

    // This should be the TypeRef type (except for )
    let type_ref_name = object_type_iter.next().unwrap().trim();

    if ["String", "Int", "Boolean"].contains(&type_ref_name) {
        return Ok((type_ref_name.to_string(), None));
    }

    // This should be the name or type of the Object
    match object_type_iter.next() {
        Some(object_type_name) => Ok((
            type_ref_name.to_string(),
            Some(object_type_name.to_string()),
        )),
        None => Ok((type_ref_name.to_string(), None)),
    }
}

#[derive(Debug)]
pub struct Parser {
    enums: Vec<EnumDef>,
    objects: Vec<ObjectDef>,
    features: Vec<FeatureDef>,
    channels: Vec<String>,
}

impl Parser {
    pub fn new(path: &Path) -> Result<Parser, FMLError> {
        let manifest = serde_yaml::from_str::<ManifestFrontEnd>(&std::fs::read_to_string(path)?)?;

        let enums: Vec<EnumDef> = manifest
            .types
            .enums
            .clone()
            .into_iter()
            .map(|t| EnumDef {
                name: t.0,
                doc: t.1.description,
                variants: t
                    .1
                    .variants
                    .iter()
                    .map(|v| VariantDef {
                        name: v.0.clone(),
                        doc: v.1.description.clone(),
                    })
                    .collect(),
            })
            .collect();

        let objects: Vec<ObjectDef> = manifest
            .types
            .objects
            .into_iter()
            .map(|t| ObjectDef {
                name: t.0,
                doc: t.1.description,
                props: t
                    .1
                    .fields
                    .into_iter()
                    .map(|v| PropDef {
                        name: v.0,
                        doc: v.1.description,
                        typ: TypeRef::try_from(v.1.variable_type).unwrap(),
                        default: match v.1.default {
                            Some(d) => json!(d),
                            None => serde_json::Value::Null,
                        },
                    })
                    .collect(),
            })
            .collect();

        // Capture the user types supplied in the manifest
        // to be able to look them up easily by name
        let mut types: HashMap<String, TypeRef> = HashMap::new();
        enums.iter().for_each(|e| {
            types.insert(e.name.to_owned(), TypeRef::Enum(e.name.to_owned()));
        });
        objects.iter().for_each(|o| {
            types.insert(o.name.to_owned(), TypeRef::Object(o.name.to_owned()));
        });

        let features: Vec<FeatureDef> = manifest
            .features
            .into_iter()
            .map(|f| FeatureDef {
                name: f.0,
                doc: f.1.description,
                props: f
                    .1
                    .variables
                    .into_iter()
                    .map(|v| PropDef {
                        name: v.0,
                        doc: v.1.description,
                        typ: match TypeRef::try_from(v.1.variable_type.to_owned()) {
                            Ok(type_ref) => type_ref,
                            Err(e) => {
                                // Try matching against the user defined types
                                match types.get(&v.1.variable_type) {
                                    Some(type_ref) => type_ref.to_owned(),
                                    None => panic!(
                                        "{}\n{} is not a valid FML type or user defined type",
                                        e, v.1.variable_type
                                    ),
                                }
                            }
                        },
                        default: json!(v.1.default),
                    })
                    .collect(),
                default: if f.1.default.is_some() {
                    Some(json!(f.1.default))
                } else {
                    None
                },
            })
            .collect();

        Ok(Parser {
            enums,
            objects,
            features,
            channels: manifest.channels,
        })
    }

    pub fn get_intermediate_representation(&self) -> Result<FeatureManifest, FMLError> {
        Ok(FeatureManifest {
            enum_defs: self.enums.clone(),
            obj_defs: self.objects.clone(),
            hints: HashMap::new(),
            feature_defs: self.features.clone(),
        })
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::error::Result;

    #[test]
    fn test_parse_from_front_end_representation() -> Result<()> {
        let path_buf = Path::new("./fixtures/fe/nimbus_features.yaml");
        let parser = Parser::new(path_buf)?;
        let ir = parser.get_intermediate_representation()?;

        // Validate parsed enums
        assert!(ir.enum_defs.len() == 1);
        assert!(ir.enum_defs.contains(parser.enums.first().unwrap()));
        let enum_def = ir.enum_defs.first().unwrap();
        assert!(enum_def.name == *"PlayerProfile");
        assert!(enum_def.doc == *"This is an enum type");
        assert!(enum_def.variants.contains(&VariantDef {
            name: "adult".to_string(),
            doc: "This represents an adult player profile".to_string()
        }));
        assert!(enum_def.variants.contains(&VariantDef {
            name: "child".to_string(),
            doc: "This represents a child player profile".to_string()
        }));

        // Validate parsed objects
        assert!(ir.obj_defs.len() == 1);
        assert!(ir.obj_defs.contains(parser.objects.first().unwrap()));
        let obj_def = ir.obj_defs.first().unwrap();
        assert!(obj_def.name == *"Button");
        assert!(obj_def.doc == *"This is a button object");
        assert!(obj_def.props.contains(&PropDef {
            name: "label".to_string(),
            doc: "This is the label for the button".to_string(),
            typ: TypeRef::String,
            default: serde_json::Value::Null,
        }));
        assert!(obj_def.props.contains(&PropDef {
            name: "color".to_string(),
            doc: "This is the color of the button".to_string(),
            typ: TypeRef::Option(Box::new(TypeRef::String)),
            default: serde_json::Value::Null,
        }));

        // Validate parsed features
        assert!(ir.feature_defs.len() == 1);
        assert!(ir.feature_defs.contains(parser.features.first().unwrap()));
        let feature_def = ir.feature_defs.first().unwrap();
        assert!(feature_def.name == *"dialog-appearance");
        assert!(feature_def.doc == *"This is the appearance of the dialog");
        let positive_button = feature_def
            .props
            .iter()
            .find(|x| x.name == "positive")
            .unwrap();
        assert!(positive_button.name == *"positive");
        assert!(positive_button.doc == *"This is a positive button");
        assert!(positive_button.typ == TypeRef::Object("Button".to_string()));
        assert!(positive_button.default.get("label").unwrap().as_str() == Some("Ok then"));
        assert!(positive_button.default.get("color").unwrap().as_str() == Some("blue"));
        let negative_button = feature_def
            .props
            .iter()
            .find(|x| x.name == "negative")
            .unwrap();
        assert!(negative_button.name == *"negative");
        assert!(negative_button.doc == *"This is a negative button");
        assert!(negative_button.typ == TypeRef::Object("Button".to_string()));
        assert!(negative_button.default.get("label").unwrap().as_str() == Some("Not this time"));
        assert!(negative_button.default.get("color").unwrap().as_str() == Some("red"));
        let background_color = feature_def
            .props
            .iter()
            .find(|x| x.name == "background-color")
            .unwrap();
        assert!(background_color.name == *"background-color");
        assert!(background_color.doc == *"This is the background color");
        assert!(background_color.typ == TypeRef::String);
        assert!(background_color.default.as_str() == Some("white"));

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_string() -> Result<()> {
        // Testing converting to TypeRef::String
        assert_eq!(
            TypeRef::try_from("String".to_string()).unwrap(),
            TypeRef::String
        );
        TypeRef::try_from("string".to_string()).unwrap_err();
        TypeRef::try_from("str".to_string()).unwrap_err();

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_int() -> Result<()> {
        // Testing converting to TypeRef::Int
        assert_eq!(TypeRef::try_from("Int".to_string()).unwrap(), TypeRef::Int);
        TypeRef::try_from("integer".to_string()).unwrap_err();
        TypeRef::try_from("int".to_string()).unwrap_err();

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_boolean() -> Result<()> {
        // Testing converting to TypeRef::Boolean
        assert_eq!(
            TypeRef::try_from("Boolean".to_string()).unwrap(),
            TypeRef::Boolean
        );
        TypeRef::try_from("boolean".to_string()).unwrap_err();
        TypeRef::try_from("bool".to_string()).unwrap_err();

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_bundletext() -> Result<()> {
        // Testing converting to TypeRef::BundleText
        assert_eq!(
            TypeRef::try_from("BundleText<test_name>".to_string()).unwrap(),
            TypeRef::BundleText("test_name".to_string())
        );
        TypeRef::try_from("bundletext(something)".to_string()).unwrap_err();
        TypeRef::try_from("BundleText()".to_string()).unwrap_err();

        // The commented out lines below represent areas we need better
        // type checking on, but are ignored for now

        // TypeRef::try_from("BundleText".to_string()).unwrap_err();
        // TypeRef::try_from("BundleText<>".to_string()).unwrap_err();
        // TypeRef::try_from("BundleText<21>".to_string()).unwrap_err();

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_bundleimage() -> Result<()> {
        // Testing converting to TypeRef::BundleImage
        assert_eq!(
            TypeRef::try_from("BundleImage<test_name>".to_string()).unwrap(),
            TypeRef::BundleImage("test_name".to_string())
        );
        TypeRef::try_from("bundleimage(something)".to_string()).unwrap_err();
        TypeRef::try_from("BundleImage()".to_string()).unwrap_err();

        // The commented out lines below represent areas we need better
        // type checking on, but are ignored for now

        // TypeRef::try_from("BundleImage".to_string()).unwrap_err();
        // TypeRef::try_from("BundleImage<>".to_string()).unwrap_err();
        // TypeRef::try_from("BundleImage<21>".to_string()).unwrap_err();

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_enum() -> Result<()> {
        // Testing converting to TypeRef::Enum
        assert_eq!(
            TypeRef::try_from("Enum<test_name>".to_string()).unwrap(),
            TypeRef::Enum("test_name".to_string())
        );
        TypeRef::try_from("enum(something)".to_string()).unwrap_err();
        TypeRef::try_from("Enum()".to_string()).unwrap_err();

        // The commented out lines below represent areas we need better
        // type checking on, but are ignored for now

        // TypeRef::try_from("Enum".to_string()).unwrap_err();
        // TypeRef::try_from("Enum<>".to_string()).unwrap_err();
        // TypeRef::try_from("Enum<21>".to_string()).unwrap_err();

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_object() -> Result<()> {
        // Testing converting to TypeRef::Object
        assert_eq!(
            TypeRef::try_from("Object<test_name>".to_string()).unwrap(),
            TypeRef::Object("test_name".to_string())
        );
        TypeRef::try_from("object(something)".to_string()).unwrap_err();
        TypeRef::try_from("Object()".to_string()).unwrap_err();

        // The commented out lines below represent areas we need better
        // type checking on, but are ignored for now

        // TypeRef::try_from("Object".to_string()).unwrap_err();
        // TypeRef::try_from("Object<>".to_string()).unwrap_err();
        // TypeRef::try_from("Object<21>".to_string()).unwrap_err();

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_list() -> Result<()> {
        // Testing converting to TypeRef::List
        assert_eq!(
            TypeRef::try_from("List<String>".to_string()).unwrap(),
            TypeRef::List(Box::new(TypeRef::String))
        );
        assert_eq!(
            TypeRef::try_from("List<Int>".to_string()).unwrap(),
            TypeRef::List(Box::new(TypeRef::Int))
        );
        assert_eq!(
            TypeRef::try_from("List<Boolean>".to_string()).unwrap(),
            TypeRef::List(Box::new(TypeRef::Boolean))
        );

        // Lists of named types are difficult to test here, since the
        // parser would parse the enums and objects first and then
        // look up the correct TypeRef::Enum or TypeRef::Object
        //
        // assert_eq!(
        //     TypeRef::try_from("List<TestEnum>".to_string()).unwrap(),
        //     TypeRef::List(Box::new(TypeRef::Enum("TestEnum".to_string())))
        // );
        // assert_eq!(
        //     TypeRef::try_from("List<TestObject>".to_string()).unwrap(),
        //     TypeRef::List(Box::new(TypeRef::Object("TestObject".to_string())))
        // );

        TypeRef::try_from("list(something)".to_string()).unwrap_err();
        TypeRef::try_from("List()".to_string()).unwrap_err();

        // The commented out lines below represent areas we need better
        // type checking on, but are ignored for now

        // TypeRef::try_from("List".to_string()).unwrap_err();
        // TypeRef::try_from("List<>".to_string()).unwrap_err();
        // TypeRef::try_from("List<21>".to_string()).unwrap_err();

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_option() -> Result<()> {
        // Testing converting to TypeRef::Option
        assert_eq!(
            TypeRef::try_from("Option<String>".to_string()).unwrap(),
            TypeRef::Option(Box::new(TypeRef::String))
        );
        assert_eq!(
            TypeRef::try_from("Option<Int>".to_string()).unwrap(),
            TypeRef::Option(Box::new(TypeRef::Int))
        );
        assert_eq!(
            TypeRef::try_from("Option<Boolean>".to_string()).unwrap(),
            TypeRef::Option(Box::new(TypeRef::Boolean))
        );

        // Option of named types are difficult to test here, since the
        // parser would parse the enums and objects first and then
        // look up the correct TypeRef::Enum or TypeRef::Object
        //
        // assert_eq!(
        //     TypeRef::try_from("Option<TestEnum>".to_string()).unwrap(),
        //     TypeRef::Option(Box::new(TypeRef::Enum("TestEnum".to_string())))
        // );
        // assert_eq!(
        //     TypeRef::try_from("Option<TestObject>".to_string()).unwrap(),
        //     TypeRef::Option(Box::new(TypeRef::Object("TestObject".to_string())))
        // );

        TypeRef::try_from("option(something)".to_string()).unwrap_err();
        TypeRef::try_from("Option(Something)".to_string()).unwrap_err();

        // The commented out lines below represent areas we need better
        // type checking on, but are ignored for now

        // TypeRef::try_from("Option".to_string()).unwrap_err();
        // TypeRef::try_from("Option<>".to_string()).unwrap_err();
        // TypeRef::try_from("Option<21>".to_string()).unwrap_err();

        Ok(())
    }

    #[test]
    fn test_convert_to_typeref_map() -> Result<()> {
        // Testing converting to TypeRef::Map
        assert_eq!(
            TypeRef::try_from("Map<String, String>".to_string()).unwrap(),
            TypeRef::StringMap(Box::new(TypeRef::String))
        );
        assert_eq!(
            TypeRef::try_from("Map<String, Int>".to_string()).unwrap(),
            TypeRef::StringMap(Box::new(TypeRef::Int))
        );
        assert_eq!(
            TypeRef::try_from("Map<String, Boolean>".to_string()).unwrap(),
            TypeRef::StringMap(Box::new(TypeRef::Boolean))
        );

        // Maps of named types are difficult to test here, since the
        // parser would parse the enums and objects first and then
        // look up the correct TypeRef::Enum or TypeRef::Object
        //
        // assert_eq!(
        //     TypeRef::try_from("Map<String, TestEnum>".to_string()).unwrap(),
        //     TypeRef::Map(Box::new(TypeRef::Enum("TestEnum".to_string())))
        // );
        // assert_eq!(
        //     TypeRef::try_from("Map<String, TestObject>".to_string()).unwrap(),
        //     TypeRef::Map(Box::new(TypeRef::Object("TestObject".to_string())))
        // );

        TypeRef::try_from("map(something)".to_string()).unwrap_err();
        TypeRef::try_from("Map(Something)".to_string()).unwrap_err();

        // The commented out lines below represent areas we need better
        // type checking on, but are ignored for now

        // TypeRef::try_from("Map".to_string()).unwrap_err();
        // TypeRef::try_from("Map<>".to_string()).unwrap_err();
        // TypeRef::try_from("Map<21>".to_string()).unwrap_err();

        Ok(())
    }
}
