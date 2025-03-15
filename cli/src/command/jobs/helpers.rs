use color_eyre::Result;
use dialoguer::console::style;
use gadget_blueprint_serde::{BoundedVec, Field, from_field, new_bounded_string};
use gadget_chain_setup::tangle::InputValue;
use serde_json;
use std::str::FromStr;
use tangle_subxt::subxt::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;

pub(crate) fn print_job_results(result_types: &[FieldType], i: usize, field: Field<AccountId32>) {
    let expected_type = result_types[i].clone();
    let print_output = match expected_type {
        FieldType::Void => "void".to_string(),
        FieldType::Bool => {
            let output: bool = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::Uint8 => {
            let output: u8 = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::Int8 => {
            let output: i8 = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::Uint16 => {
            let output: u16 = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::Int16 => {
            let output: i16 = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::Uint32 => {
            let output: u32 = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::Int32 => {
            let output: i32 = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::Uint64 => {
            let output: u64 = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::Int64 => {
            let output: i64 = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::String => {
            let output: String = from_field(field).unwrap();
            output.to_string()
        }
        FieldType::Optional(_field_type) => {
            let output: Option<FieldType> = from_field(field.clone()).unwrap();
            if output.is_some() {
                let output: FieldType = output.unwrap();
                print_job_results(&[output], 0, field);
                "Some".to_string()
            } else {
                "None".to_string()
            }
        }
        FieldType::Array(_, _field_type) => {
            let output: Vec<FieldType> = from_field(field.clone()).unwrap();
            for (i, inner_type) in output.iter().enumerate() {
                print_job_results(&[inner_type.clone()], i, field.clone());
            }
            "Array".to_string()
        }
        FieldType::List(_field_type) => {
            let output: BoundedVec<FieldType> = from_field(field.clone()).unwrap();
            for (i, inner_type) in output.0.iter().enumerate() {
                print_job_results(&[inner_type.clone()], i, field.clone());
            }
            "List".to_string()
        }
        FieldType::Struct(_bounded_vec) => {
            let output: BoundedVec<FieldType> = from_field(field.clone()).unwrap();
            for (i, inner_type) in output.0.iter().enumerate() {
                print_job_results(&[inner_type.clone()], i, field.clone());
            }
            "Struct".to_string()
        }
        FieldType::AccountId => {
            let output: AccountId32 = from_field(field).unwrap();
            output.to_string()
        }
    };
    println!(
        "{}: {}",
        style(format!("Output {}", i + 1)).green().bold(),
        style(format!("{:?}", print_output)).green()
    );
}

/// Load job arguments from a JSON file
///
/// # Arguments
///
/// * `file_path` - Path to the JSON file
/// * `param_types` - Types of parameters expected
///
/// # Returns
///
/// A vector of input values parsed from the file.
///
/// # Errors
///
/// Returns an error if:
/// * File not found
/// * File content is not valid JSON
/// * JSON is not an array
/// * Number of arguments doesn't match expected parameters
/// * Arguments don't match expected types
#[allow(clippy::too_many_lines)]
pub(crate) fn load_job_args_from_file(
    file_path: &str,
    param_types: &[FieldType],
) -> Result<Vec<InputValue>> {
    use std::fs;
    use std::path::Path;

    let path = Path::new(file_path);
    if !path.exists() {
        return Err(color_eyre::eyre::eyre!(
            "Parameters file not found: {}",
            file_path
        ));
    }

    let content = fs::read_to_string(path)?;
    let json_values: serde_json::Value = serde_json::from_str(&content)?;

    if !json_values.is_array() {
        return Err(color_eyre::eyre::eyre!(
            "Job arguments must be provided as a JSON array"
        ));
    }

    let args = json_values.as_array().unwrap();

    if args.len() != param_types.len() {
        return Err(color_eyre::eyre::eyre!(
            "Expected {} arguments but got {}",
            param_types.len(),
            args.len()
        ));
    }

    // Parse each argument according to the expected parameter type
    let mut input_values = Vec::new();
    for (i, (arg, param_type)) in args.iter().zip(param_types.iter()).enumerate() {
        let input_value = match param_type {
            FieldType::Uint8 => {
                let value = arg.as_u64().ok_or_else(|| {
                    color_eyre::eyre::eyre!("Argument {} must be a u8 integer", i)
                })?;
                if value > u64::from(u8::MAX) {
                    return Err(color_eyre::eyre::eyre!("Argument {} exceeds u8 range", i));
                }
                InputValue::Uint8(
                    u8::try_from(value)
                        .map_err(|_| color_eyre::eyre::eyre!("Failed to convert to u8"))?,
                )
            }
            FieldType::Uint16 => {
                let value = arg.as_u64().ok_or_else(|| {
                    color_eyre::eyre::eyre!("Argument {} must be a u16 integer", i)
                })?;
                if value > u64::from(u16::MAX) {
                    return Err(color_eyre::eyre::eyre!("Argument {} exceeds u16 range", i));
                }
                InputValue::Uint16(
                    u16::try_from(value)
                        .map_err(|_| color_eyre::eyre::eyre!("Failed to convert to u16"))?,
                )
            }
            FieldType::Uint32 => {
                let value = arg.as_u64().ok_or_else(|| {
                    color_eyre::eyre::eyre!("Argument {} must be a u32 integer", i)
                })?;
                if value > u64::from(u32::MAX) {
                    return Err(color_eyre::eyre::eyre!("Argument {} exceeds u32 range", i));
                }
                InputValue::Uint32(
                    u32::try_from(value)
                        .map_err(|_| color_eyre::eyre::eyre!("Failed to convert to u32"))?,
                )
            }
            FieldType::Uint64 => {
                let value = arg.as_u64().ok_or_else(|| {
                    color_eyre::eyre::eyre!("Argument {} must be a u64 integer", i)
                })?;
                InputValue::Uint64(value)
            }
            FieldType::Int8 => {
                let value = arg.as_i64().ok_or_else(|| {
                    color_eyre::eyre::eyre!("Argument {} must be an i8 integer", i)
                })?;
                if value < i64::from(i8::MIN) || value > i64::from(i8::MAX) {
                    return Err(color_eyre::eyre::eyre!("Argument {} exceeds i8 range", i));
                }
                InputValue::Int8(
                    i8::try_from(value)
                        .map_err(|_| color_eyre::eyre::eyre!("Failed to convert to i8"))?,
                )
            }
            FieldType::Int16 => {
                let value = arg.as_i64().ok_or_else(|| {
                    color_eyre::eyre::eyre!("Argument {} must be an i16 integer", i)
                })?;
                if value < i64::from(i16::MIN) || value > i64::from(i16::MAX) {
                    return Err(color_eyre::eyre::eyre!("Argument {} exceeds i16 range", i));
                }
                InputValue::Int16(
                    i16::try_from(value)
                        .map_err(|_| color_eyre::eyre::eyre!("Failed to convert to i16"))?,
                )
            }
            FieldType::Int32 => {
                let value = arg.as_i64().ok_or_else(|| {
                    color_eyre::eyre::eyre!("Argument {} must be an i32 integer", i)
                })?;
                if value < i64::from(i32::MIN) || value > i64::from(i32::MAX) {
                    return Err(color_eyre::eyre::eyre!("Argument {} exceeds i32 range", i));
                }
                InputValue::Int32(
                    i32::try_from(value)
                        .map_err(|_| color_eyre::eyre::eyre!("Failed to convert to i32"))?,
                )
            }
            FieldType::Int64 => {
                let value = arg.as_i64().ok_or_else(|| {
                    color_eyre::eyre::eyre!("Argument {} must be an i64 integer", i)
                })?;
                InputValue::Int64(value)
            }
            FieldType::Bool => {
                let value = arg
                    .as_bool()
                    .ok_or_else(|| color_eyre::eyre::eyre!("Argument {} must be a boolean", i))?;
                InputValue::Bool(value)
            }
            FieldType::String => {
                let value = arg
                    .as_str()
                    .ok_or_else(|| color_eyre::eyre::eyre!("Argument {} must be a string", i))?;
                InputValue::String(new_bounded_string(value.to_string()))
            }
            _ => {
                return Err(color_eyre::eyre::eyre!(
                    "Unsupported parameter type: {:?}",
                    param_type
                ));
            }
        };

        input_values.push(input_value);
    }

    Ok(input_values)
}

/// Prompt the user for job parameters based on the parameter types
#[allow(clippy::too_many_lines)]
pub(crate) fn prompt_for_job_params(param_types: &[FieldType]) -> Result<Vec<InputValue>> {
    use dialoguer::Input;

    let mut args = Vec::new();

    for (i, param_type) in param_types.iter().enumerate() {
        println!("Parameter {}: {:?}", i + 1, param_type);

        match param_type {
            FieldType::Uint8 => {
                let value: u8 = Input::new()
                    .with_prompt(format!("Enter u8 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Uint8(value));
            }
            FieldType::Uint16 => {
                let value: u16 = Input::new()
                    .with_prompt(format!("Enter u16 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Uint16(value));
            }
            FieldType::Uint32 => {
                let value: u32 = Input::new()
                    .with_prompt(format!("Enter u32 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Uint32(value));
            }
            FieldType::Uint64 => {
                let value: u64 = Input::new()
                    .with_prompt(format!("Enter u64 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Uint64(value));
            }
            FieldType::Int8 => {
                let value: i8 = Input::new()
                    .with_prompt(format!("Enter i8 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Int8(value));
            }
            FieldType::Int16 => {
                let value: i16 = Input::new()
                    .with_prompt(format!("Enter i16 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Int16(value));
            }
            FieldType::Int32 => {
                let value: i32 = Input::new()
                    .with_prompt(format!("Enter i32 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Int32(value));
            }
            FieldType::Int64 => {
                let value: i64 = Input::new()
                    .with_prompt(format!("Enter i64 value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::Int64(value));
            }
            FieldType::Bool => {
                let value: bool = Input::new()
                    .with_prompt(format!(
                        "Enter boolean value (true/false) for parameter {}",
                        i + 1
                    ))
                    .interact()?;
                args.push(InputValue::Bool(value));
            }
            FieldType::String => {
                let value: String = Input::new()
                    .with_prompt(format!("Enter string value for parameter {}", i + 1))
                    .interact()?;
                args.push(InputValue::String(new_bounded_string(value)));
            }
            FieldType::Void => {
                println!("Void parameter, no input required");
            }
            FieldType::Optional(field_type) => {
                use dialoguer::Confirm;

                let include_value = Confirm::new()
                    .with_prompt(format!("Include a value for optional parameter {}?", i + 1))
                    .default(false)
                    .interact()?;

                if include_value {
                    // Recursively prompt for the inner type
                    let inner_values = prompt_for_job_params(&[*field_type.clone()])?;
                    if let Some(inner_value) = inner_values.first() {
                        args.push(InputValue::Optional(
                            FieldType::String,
                            Box::new(Some(inner_value.clone())),
                        ));
                    }
                } else {
                    args.push(InputValue::Optional(FieldType::String, Box::new(None)));
                }
            }
            FieldType::Array(size, field_type) => {
                println!("Enter {} values for array parameter {}", size, i + 1);
                let mut array_values = Vec::new();

                for j in 0..*size {
                    println!("Array element {} of {}", j + 1, size);
                    let inner_values = prompt_for_job_params(&[*field_type.clone()])?;
                    if let Some(inner_value) = inner_values.first() {
                        array_values.push(inner_value.clone());
                    }
                }

                let values = BoundedVec(array_values);
                args.push(InputValue::Array(*field_type.clone(), values));
            }
            FieldType::List(field_type) => {
                use dialoguer::Input;

                let count: usize = Input::new()
                    .with_prompt(format!("How many elements for list parameter {}?", i + 1))
                    .default(0)
                    .interact()?;

                let mut list_values = Vec::new();

                for j in 0..count {
                    println!("List element {} of {}", j + 1, count);
                    let inner_values = prompt_for_job_params(&[*field_type.clone()])?;
                    if let Some(inner_value) = inner_values.first() {
                        list_values.push(inner_value.clone());
                    }
                }

                let values = BoundedVec(list_values);
                args.push(InputValue::List(*field_type.clone(), values));
            }
            FieldType::Struct(_bounded_vec) => {
                todo!();
            }
            FieldType::AccountId => {
                let value: String = Input::new()
                    .with_prompt(format!(
                        "Enter AccountId for parameter {} (SS58 format)",
                        i + 1
                    ))
                    .interact()?;

                // Parse the account ID from the string
                match AccountId32::from_str(&value) {
                    Ok(account_id) => args.push(InputValue::AccountId(account_id)),
                    Err(_) => return Err(color_eyre::eyre::eyre!("Invalid AccountId format")),
                }
            }
        }
    }

    Ok(args)
}
