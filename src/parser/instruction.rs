/**
 * Copyright 2021 Rigetti Computing
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 **/
use nom::{
    combinator::all_consuming,
    multi::{many0, many1},
    sequence::{delimited, preceded},
};

use crate::{
    instruction::{ArithmeticOperator, Instruction},
    token,
};

use super::{
    command, common,
    error::{Error, ErrorKind},
    gate,
    lexer::{Command, Token},
    ParserInput, ParserResult,
};

/// Parse the next instructon from the input, skipping past leading newlines, comments, and semicolons.
pub fn parse_instruction(input: ParserInput) -> ParserResult<Instruction> {
    let (input, _) = common::skip_newlines_and_comments(input)?;
    match input.split_first() {
        None => Err(nom::Err::Error(Error {
            input,
            error: ErrorKind::EndOfInput,
        })),
        Some((Token::Command(command), remainder)) => {
            match command {
                Command::Add => command::parse_arithmetic(ArithmeticOperator::Add, remainder),
                // Command::And => {}
                Command::Capture => command::parse_capture(remainder),
                // Command::Convert => {}
                Command::Declare => command::parse_declare(remainder),
                Command::DefCal => command::parse_defcal(remainder),
                Command::DefCircuit => command::parse_defcircuit(remainder),
                Command::DefFrame => command::parse_defframe(remainder),
                // Command::DefGate => Ok((remainder, cut(parse_command_defgate))),
                Command::DefWaveform => command::parse_defwaveform(remainder),
                Command::Delay => command::parse_delay(remainder),
                Command::Div => command::parse_arithmetic(ArithmeticOperator::Divide, remainder),
                // Command::Eq => {}
                // Command::Exchange => {}
                // Command::Fence => {}
                // Command::GE => {}
                // Command::GT => {}
                Command::Halt => Ok((remainder, Instruction::Halt)),
                // Command::Include => {}
                // Command::Ior => {}
                Command::Jump => command::parse_jump(remainder),
                Command::JumpUnless => command::parse_jump_unless(remainder),
                Command::JumpWhen => command::parse_jump_when(remainder),
                Command::Label => command::parse_label(remainder),
                // Command::LE => {}
                Command::Load => command::parse_load(remainder),
                // Command::LT => {}
                Command::Measure => command::parse_measurement(remainder),
                Command::Move => command::parse_move(remainder),
                Command::Exchange => command::parse_exchange(remainder),
                Command::Mul => command::parse_arithmetic(ArithmeticOperator::Multiply, remainder),
                // Command::Neg => {}
                // Command::Nop => {}
                // Command::Not => {}
                Command::Pragma => command::parse_pragma(remainder),
                Command::Pulse => command::parse_pulse(input),
                Command::RawCapture => command::parse_raw_capture(remainder),
                // Command::Reset => {}
                // Command::SetFrequency => {}
                // Command::SetPhase => {}
                // Command::SetScale => {}
                // Command::ShiftFrequency => {}
                // Command::ShiftPhase => {}
                Command::Store => command::parse_store(remainder),
                Command::Sub => command::parse_arithmetic(ArithmeticOperator::Subtract, remainder),
                // Command::Wait => {}
                // Command::Xor => {}
                _ => Err(nom::Err::Failure(Error {
                    input: &input[..1],
                    error: ErrorKind::UnsupportedInstruction,
                })),
            }
            .map_err(|err| {
                nom::Err::Failure(Error {
                    input: &input[..1],
                    error: ErrorKind::InvalidCommand {
                        command: command.clone(),
                        error: format!("{}", err),
                    },
                })
            })
        }
        Some((Token::NonBlocking, _)) => command::parse_pulse(input),
        Some((Token::Identifier(_), _)) | Some((Token::Modifier(_), _)) => gate::parse_gate(input),
        Some((_, _)) => Err(nom::Err::Failure(Error {
            input: &input[..1],
            error: ErrorKind::NotACommandOrGate,
        })),
    }
}

/// Parse all instructions from the input, trimming leading and trailing newlines and comments.
/// Returns an error if it does not reach the end of input.
pub fn parse_instructions(input: ParserInput) -> ParserResult<Vec<Instruction>> {
    all_consuming(delimited(
        common::skip_newlines_and_comments,
        many0(parse_instruction),
        common::skip_newlines_and_comments,
    ))(input)
}

/// Parse a block of indented "block instructions."
pub fn parse_block(input: ParserInput) -> ParserResult<Vec<Instruction>> {
    many1(parse_block_instruction)(input)
}

/// Parse a single indented "block instruction."
pub fn parse_block_instruction<'a>(input: ParserInput<'a>) -> ParserResult<'a, Instruction> {
    preceded(
        token!(NewLine),
        preceded(token!(Indentation), parse_instruction),
    )(input)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        expression::Expression,
        instruction::{
            ArithmeticOperand, ArithmeticOperator, AttributeValue, FrameIdentifier, Instruction,
            MemoryReference, Qubit, WaveformInvocation,
        },
        make_test, real,
    };
    use crate::{instruction::Calibration, parser::lexer::lex};

    use super::parse_instructions;

    make_test!(
        semicolons_are_newlines,
        parse_instructions,
        "X 0; Y 1\nZ 2",
        vec![
            Instruction::Gate {
                name: "X".to_owned(),
                parameters: vec![],
                qubits: vec![Qubit::Fixed(0)],
                modifiers: vec![],
            },
            Instruction::Gate {
                name: "Y".to_owned(),
                parameters: vec![],
                qubits: vec![Qubit::Fixed(1)],
                modifiers: vec![],
            },
            Instruction::Gate {
                name: "Z".to_owned(),
                parameters: vec![],
                qubits: vec![Qubit::Fixed(2)],
                modifiers: vec![],
            },
        ]
    );

    make_test!(
        arithmetic,
        parse_instructions,
        "ADD ro 2\nMUL ro 1.0\nSUB ro[1] -3\nDIV ro[1] -1.0\nADD ro[1] ro[2]",
        vec![
            Instruction::Arithmetic {
                operator: ArithmeticOperator::Add,
                destination: ArithmeticOperand::MemoryReference(MemoryReference {
                    name: "ro".to_owned(),
                    index: 0
                }),
                source: ArithmeticOperand::LiteralInteger(2),
            },
            Instruction::Arithmetic {
                operator: ArithmeticOperator::Multiply,
                destination: ArithmeticOperand::MemoryReference(MemoryReference {
                    name: "ro".to_owned(),
                    index: 0
                }),
                source: ArithmeticOperand::LiteralReal(1.0),
            },
            Instruction::Arithmetic {
                operator: ArithmeticOperator::Subtract,
                destination: ArithmeticOperand::MemoryReference(MemoryReference {
                    name: "ro".to_owned(),
                    index: 1
                }),
                source: ArithmeticOperand::LiteralInteger(-3),
            },
            Instruction::Arithmetic {
                operator: ArithmeticOperator::Divide,
                destination: ArithmeticOperand::MemoryReference(MemoryReference {
                    name: "ro".to_owned(),
                    index: 1
                }),
                source: ArithmeticOperand::LiteralReal(-1f64),
            },
            Instruction::Arithmetic {
                operator: ArithmeticOperator::Add,
                destination: ArithmeticOperand::MemoryReference(MemoryReference {
                    name: "ro".to_owned(),
                    index: 1
                }),
                source: ArithmeticOperand::MemoryReference(MemoryReference {
                    name: "ro".to_owned(),
                    index: 2
                }),
            }
        ]
    );

    make_test!(
        capture_instructions,
        parse_instructions,
        "CAPTURE 0 \"rx\" my_custom_waveform ro\nRAW-CAPTURE 0 1 \"rx\" 2e9 ro",
        vec![
            Instruction::Capture {
                frame: FrameIdentifier {
                    name: "rx".to_owned(),
                    qubits: vec![Qubit::Fixed(0)]
                },
                waveform: Box::new(WaveformInvocation {
                    name: "my_custom_waveform".to_owned(),
                    parameters: HashMap::new()
                }),
                memory_reference: MemoryReference {
                    name: "ro".to_owned(),
                    index: 0
                }
            },
            Instruction::RawCapture {
                frame: FrameIdentifier {
                    name: "rx".to_owned(),
                    qubits: vec![Qubit::Fixed(0), Qubit::Fixed(1)]
                },
                duration: Expression::Number(real![2e9]),
                memory_reference: MemoryReference {
                    name: "ro".to_owned(),
                    index: 0
                }
            }
        ]
    );

    make_test!(comment, parse_instructions, "# Questions:\n\n\n", vec![]);

    make_test!(
        comment_and_gate,
        parse_instructions,
        "# Questions:\nX 0",
        vec![Instruction::Gate {
            name: "X".to_owned(),
            parameters: vec![],
            qubits: vec![Qubit::Fixed(0)],
            modifiers: vec![],
        }]
    );

    make_test!(
        comment_after_block,
        parse_instructions,
        "DEFFRAME 0 \"ro_rx\":\n\tDIRECTION: \"rx\"\n\n# (Pdb) settings.gates[GateID(name=\"x180\", targets=(0,))]\n\n",
        vec![Instruction::FrameDefinition {
            identifier: FrameIdentifier { name: "ro_rx".to_owned(), qubits: vec![Qubit::Fixed(0)] },
            attributes: [("DIRECTION".to_owned(), AttributeValue::String("rx".to_owned()))].iter().cloned().collect()
        }]);

    make_test!(
        simple_gate,
        parse_instructions,
        "RX 0",
        vec![Instruction::Gate {
            name: "RX".to_owned(),
            parameters: vec![],
            qubits: vec![Qubit::Fixed(0)],
            modifiers: vec![],
        }]
    );

    make_test!(
        parametric_gate,
        parse_instructions,
        "RX(pi) 10",
        vec![Instruction::Gate {
            name: "RX".to_owned(),
            parameters: vec![Expression::PiConstant],
            qubits: vec![Qubit::Fixed(10)],
            modifiers: vec![],
        }]
    );

    make_test!(
        parametric_calibration,
        parse_instructions,
        "DEFCAL RX(%theta) %qubit:\n\tPULSE 1 \"xy\" custom_waveform(a: 1)",
        vec![Instruction::CalibrationDefinition(Box::new(Calibration {
            name: "RX".to_owned(),
            parameters: vec![Expression::Variable("theta".to_owned())],
            qubits: vec![Qubit::Variable("qubit".to_owned())],
            modifiers: vec![],
            instructions: vec![Instruction::Pulse {
                blocking: true,
                frame: FrameIdentifier {
                    name: "xy".to_owned(),
                    qubits: vec![Qubit::Fixed(1)]
                },
                waveform: Box::new(WaveformInvocation {
                    name: "custom_waveform".to_owned(),
                    parameters: [("a".to_owned(), Expression::Number(crate::real![1f64]))]
                        .iter()
                        .cloned()
                        .collect()
                })
            }]
        }))]
    );

    make_test!(
        frame_definition,
        parse_instructions,
        "DEFFRAME 0 \"rx\":\n\tINITIAL-FREQUENCY: 2e9",
        vec![Instruction::FrameDefinition {
            identifier: FrameIdentifier {
                name: "rx".to_owned(),
                qubits: vec![Qubit::Fixed(0)]
            },
            attributes: [(
                "INITIAL-FREQUENCY".to_owned(),
                AttributeValue::Expression(Expression::Number(crate::real![2e9]))
            )]
            .iter()
            .cloned()
            .collect()
        }]
    );

    make_test!(
        control_flow,
        parse_instructions,
        "LABEL @hello\nJUMP @hello\nJUMP-WHEN @hello ro",
        vec![
            Instruction::Label("hello".to_owned()),
            Instruction::Jump {
                target: "hello".to_owned()
            },
            Instruction::JumpWhen {
                target: "hello".to_owned(),
                condition: MemoryReference {
                    name: "ro".to_owned(),
                    index: 0
                }
            }
        ]
    );

    make_test!(
        pulse,
        parse_instructions,
        "PULSE 0 \"xy\" custom\nNONBLOCKING PULSE 0 \"xy\" custom",
        vec![
            Instruction::Pulse {
                blocking: true,
                frame: FrameIdentifier {
                    name: "xy".to_owned(),
                    qubits: vec![Qubit::Fixed(0)]
                },
                waveform: Box::new(WaveformInvocation {
                    name: "custom".to_owned(),
                    parameters: HashMap::new()
                })
            },
            Instruction::Pulse {
                blocking: false,
                frame: FrameIdentifier {
                    name: "xy".to_owned(),
                    qubits: vec![Qubit::Fixed(0)]
                },
                waveform: Box::new(WaveformInvocation {
                    name: "custom".to_owned(),
                    parameters: HashMap::new()
                })
            }
        ]
    );

    make_test!(
        moveit,
        parse_instructions,
        "MOVE a 1.0",
        vec![Instruction::Move {
            destination: ArithmeticOperand::MemoryReference(MemoryReference {
                name: "a".to_owned(),
                index: 0
            }),
            source: ArithmeticOperand::LiteralReal(1.0)
        }]
    );
}
