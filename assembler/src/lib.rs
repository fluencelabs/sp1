use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use curta_core::program::opcodes::*;
use pest::Parser;
use pest_derive::*;
use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "grammar/assembly.pest"]
pub struct AssemblyParser;

pub fn assemble(input: &str) -> Result<Vec<u8>, String> {
    let parsed = AssemblyParser::parse(Rule::assembly, input).unwrap();

    // First pass: Record label locations
    let mut label_locations = HashMap::new();
    let mut pc = 0;
    for pair in parsed.clone() {
        match pair.as_rule() {
            Rule::label => {
                let label_name = pair.as_str().trim().trim_end_matches(':');
                label_locations.insert(label_name, pc);
            }
            Rule::instruction => {
                pc += 1;
            }
            _ => {}
        }
    }

    // Second pass: Generate machine code and replace labels with PC locations
    let mut vec: Vec<u8> = Vec::new();
    let mut pc = 0;
    for pair in parsed {
        match pair.as_rule() {
            Rule::instruction => {
                let mut inner_pairs = pair.into_inner();
                let mnemonic = inner_pairs.next().unwrap().as_str();
                let mut operands: Vec<i32> = inner_pairs
                    .filter_map(|p| {
                        if p.as_rule() == Rule::WHITESPACE {
                            return None;
                        }
                        let op_str = p.as_str();
                        let ret = if op_str.ends_with("(fp)") {
                            // Extract the numeric value from the string and convert to i32
                            op_str.trim_end_matches("(fp)").parse::<i32>().unwrap()
                        } else if label_locations.contains_key(op_str) {
                            // If operand is a label reference, replace with the `pc` offset
                            *label_locations.get(op_str).unwrap() - pc
                        } else {
                            // Otherwise, use the operand as-is
                            op_str.parse::<i32>().unwrap()
                        };
                        Some(ret)
                    })
                    .collect();

                // Convert mnemonic to opcode
                let opcode = match mnemonic {
                    // Core CPU
                    "lw" => Opcode::ADDI,
                    "jal" => Opcode::JAL,
                    "jali" => Opcode::JALI,
                    // "beq" | "beqi" => BEQ,
                    // "bne" | "bnei" => BNE,
                    // "stop" => STOP,

                    // U32 ALU
                    "add" => Opcode::ADD,
                    "sub" => Opcode::SUB,
                    "xor" => Opcode::XOR,
                    "and" => Opcode::AND,
                    "shl" => Opcode::SLL,

                    // // Native field
                    // "feadd" => ADD,
                    // "fesub" => SUB,
                    _ => panic!("Unknown mnemonic: {}", mnemonic),
                };

                // Insert zero operands if necessary
                match mnemonic {
                    "lw" => {
                        // (a, c, 0, 0, 0)
                        operands.extend(vec![0; 3]);
                    }
                    "stop" => {
                        // (0, 0, 0, 0, 0)
                        operands.extend(vec![0; 5]);
                    }
                    "addi" | "subi" | "muli" | "divi" | "lti" | "shli" | "shri" | "beqi"
                    | "bnei" | "andi" | "ori" | "xori" => {
                        // (a, b, c, 0, 1)
                        operands.extend(vec![0, 1]);
                    }
                    _ => {
                        // (a, b, c, 0, 0)
                        operands.extend(vec![0; 2]);
                    }
                };

                // Write opcode and operands
                vec.write_u32::<LittleEndian>(opcode as u32).unwrap();
                for operand in operands {
                    vec.write_i32::<LittleEndian>(operand).unwrap();
                }
                pc += 1;
            }
            _ => {}
        }
    }

    Ok(vec)
}