#![warn(clippy::all, clippy::pedantic)]

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use clap::Parser;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};

use giggle_flop::instruction::instruction::Instruction;

// TODO: Look into adding a .DATA directive...
// TODO: Writing to disk...

type Address = u32;

const DEFAULT_OUTPUT_PATH: &str = "a";
// Borrowed from tree-sitter-c -- https://github.com/tree-sitter/tree-sitter-c/blob/1aafaff4d26dac5a36dd3495be33e1c20161d761/grammar.js#L1336
const LINE_COMMENT_REGEX: &str = r"//(\\+(.|\r?\n)|[^\\\n])*";
const LABEL_REGEX: &str = r"^\s*[a-zA-Z][\w]+:";

const NUM_GEN_REGS: usize = 16;
const NUM_FLOAT_REGS: usize = 16;
const MAX_IMMEDIATE_VAL: u32 = 2u32.pow(21);

// TODO: Maybe refactor this so we load it into a hashmap?
const TYPE_0_INSTRS: &[&str] = &["RET", "HALT"];
const TYPE_1_INSTRS: &[&str] = &[
    "CALL", "JE", "JNE", "JGT", "JLT", "JGTE", "JLTE", "IJE", "IJNE", "IJGT", "IJLT", "IJGTE",
    "IJLTE",
];
const TYPE_2_INSTRS: &[&str] = &["CMP8", "CMP16", "CMP32", "LDIN8", "LDIN16", "LDIN32"];
const TYPE_3_INSTRS: &[&str] = &["CMPF"];
const TYPE_4_INSTRS: &[&str] = &[
    "LD8", "LD16", "LD32", "LDI8", "LDI16", "LDI32", "ST8", "ST16", "ST32", "ADDIM",
];
const TYPE_5_INSTRS: &[&str] = &[
    "ADDI", "SUBI", "MULI", "DIVI", "MODI", "RBSI", "XORI", "ANDI", "ORI", "ADDU", "SUBU", "MULU",
    "DIVU", "MODU",
];
const TYPE_6_INSTRS: &[&str] = &["ADDF", "SUBF", "MULF", "DIVF"];

const ALL_INSTR_TYPES: &[&[&str]] = &[
    TYPE_0_INSTRS,
    TYPE_1_INSTRS,
    TYPE_2_INSTRS,
    TYPE_3_INSTRS,
    TYPE_4_INSTRS,
    TYPE_5_INSTRS,
    TYPE_6_INSTRS,
];

const INSTR_TYPE_0_REGEX: &str = r"(?P<opcode>[a-zA-Z]+)";
const INSTR_TYPE_1_REGEX: &str =
    r"(?P<opcode>[a-zA-Z]+)\s+((?P<immediate_val>\d+)|(?P<immediate_label>[a-zA-Z][\w]+))";
const INSTR_TYPE_2_REGEX: &str =
    r"(?P<opcode>[a-zA-Z0-9]+)\s+(?P<reg_1>R\d+)\s*,\s*(?P<reg_2>R\d+)";
const INSTR_TYPE_3_REGEX: &str =
    r"(?P<opcode>[a-zA-Z0-9]+)\s+(?P<reg_1>F\d+)\s*,\s*(?P<reg_2>F\d+)";
const INSTR_TYPE_4_REGEX: &str = r"(?P<opcode>[a-zA-Z0-9]+)\s+(?P<reg_1>R\d+)\s*,\s*((?P<immediate_val>\d+)|(?P<immediate_label>[a-zA-Z][\w]+))";
const INSTR_TYPE_5_REGEX: &str =
    r"(?P<opcode>[a-zA-Z0-9]+)\s+(?P<reg_1>R\d+)\s*,\s*(?P<reg_2>R\d+)\s*,\s*(?P<reg_3>R\d+)";
const INSTR_TYPE_6_REGEX: &str =
    r"(?P<opcode>[a-zA-Z0-9]+)\s+(?P<reg_1>F\d+)\s*,\s*(?P<reg_2>F\d+)\s*,\s*(?P<reg_3>F\d+)";

const TYPE_FIELD_WIDTH: usize = 3;
const REG_FIELD_WIDTH: usize = 4;
#[allow(dead_code)]
const TYPE_0_OPCODE_FIELD_WIDTH: usize = 1;
const TYPE_1_OPCODE_FIELD_WIDTH: usize = 4;
const TYPE_2_OPCODE_FIELD_WIDTH: usize = 3;
const TYPE_3_OPCODE_FIELD_WIDTH: usize = 1;
const TYPE_4_OPCODE_FIELD_WIDTH: usize = 4;
const TYPE_5_OPCODE_FIELD_WIDTH: usize = 4;
const TYPE_6_OPCODE_FIELD_WIDTH: usize = 2;
const INSTR_WIDTH_BITS: Address = 32;
const INSTR_START_ADDR: Address = 0;

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
enum RegisterGroup {
    General,
    FloatingPoint,
}

// #[derive(Debug, Clone, Eq, PartialEq, Copy)]
// enum Instruction {
//     Type0 {
//         opcode: u32,
//     }, // No arguments
//     Type1 {
//         opcode: u32,
//         immediate: u32,
//     }, // One immediate argument
//     Type2 {
//         opcode: u32,
//         reg_1: usize,
//         reg_2: usize,
//     }, // Two general purpose register arguments
//     Type3 {
//         opcode: u32,
//         freg_1: usize,
//         freg_2: usize,
//     }, // Two floating point register arguments
//     Type4 {
//         opcode: u32,
//         reg_1: usize,
//         immediate: u32,
//     }, // One general purpose register argument, one immediate
//     Type5 {
//         opcode: u32,
//         reg_1: usize,
//         reg_2: usize,
//         reg_3: usize,
//     }, // Three general purpose register arguments
//     Type6 {
//         opcode: u32,
//         freg_1: usize,
//         freg_2: usize,
//         freg_3: usize,
//     }, // Three floating point register arguments
// }

#[derive(Parser, Debug)]
struct AssemblerArgs {
    input_file: PathBuf,
    #[arg(long, short, help = "Path to store the output file")]
    output_path: Option<PathBuf>,
}

#[derive(clap::Args, Debug)]
#[command(version, about, long_about = None)]
struct AssemblerOptions {
    input_path: PathBuf,
    output_path: Option<PathBuf>,
}

impl From<AssemblerArgs> for AssemblerOptions {
    fn from(value: AssemblerArgs) -> Self {
        AssemblerOptions {
            input_path: value.input_file,
            output_path: value.output_path,
        }
    }
}

fn read_input(path: &Path) -> Result<String> {
    // try to canonicalize the path
    let path = path.canonicalize()?;
    // try to read the file in
    let data = std::fs::read_to_string(path)?;
    // return
    Ok(data)
}

fn strip_comments(conts: &str) -> (String, HashSet<usize>) {
    let line_comment_regex = Regex::new(LINE_COMMENT_REGEX).unwrap();
    let mut cleaned = String::new();
    let mut removed_lines = HashSet::new();

    for (line_num, line) in conts.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        if let Some(caps) = line_comment_regex.captures(line) {
            if let Some(cap) = caps.get(0) {
                let removed = line.replace(cap.as_str(), "");
                if removed.is_empty() {
                    removed_lines.insert(line_num + 1);
                } else {
                    cleaned += &format!("{}\n", line.replace(cap.as_str(), "").trim());
                }
            }
        } else {
            cleaned += &format!("{}\n", line.trim());
        }
    }

    (cleaned, removed_lines)
}

fn get_label_to_addr_map(conts: &str) -> HashMap<String, Address> {
    let label_regex = Regex::new(LABEL_REGEX).unwrap();
    let mut curr_addr = INSTR_START_ADDR;
    let mut map = HashMap::new();

    for line in conts.lines() {
        if let Some(cap) = label_regex.captures(line) {
            if let Some(label) = cap.get(0) {
                let label = label.as_str().replace(':', "");
                map.insert(label, curr_addr);
            }
        } else {
            curr_addr += INSTR_WIDTH_BITS;
        }
    }

    map
}

fn get_instr_type(instr: &str) -> Result<usize> {
    let opcode: String = if instr.contains([' ', ',']) {
        let splits: Vec<&str> = instr.split(&[' ', ',']).collect();
        match splits.first() {
            Some(word) => (*word).to_string(),
            None => {
                return Err(anyhow!("Unable to determine instruction type: {instr}"));
            }
        }
    } else {
        instr.to_owned()
    };

    let type_check = |instr_list: &[&str]| -> bool {
        instr_list
            .iter()
            .any(|instr_name| instr_name.eq_ignore_ascii_case(&opcode))
    };

    if let Some(instr_type) =
        ALL_INSTR_TYPES
            .iter()
            .enumerate()
            .find_map(|(i, instrs)| if type_check(instrs) { Some(i) } else { None })
    {
        Ok(instr_type)
    } else {
        Err(anyhow!(
            "Unable to determine instruction type of given opcode: {opcode}"
        ))
    }
}

fn parse_opcode(
    instr: &str,
    instr_caps: &Captures<'_>,
    instr_type: usize,
    line_num: usize,
) -> Result<u32> {
    let Some(opcode) = instr_caps.name("opcode") else {
        return Err(anyhow!(
            "Line {line_num}: Parsing failure. Invalid Type {instr_type} instruction: {instr}"
        ));
    };

    if instr_type >= ALL_INSTR_TYPES.len() {
        return Err(anyhow!("Invalid instruction type: {instr_type}"));
    }

    let idx = ALL_INSTR_TYPES[instr_type]
        .iter()
        .enumerate()
        .find_map(|(i, known_opcode)| {
            if known_opcode.eq_ignore_ascii_case(opcode.as_str()) {
                Some(i)
            } else {
                None
            }
        });

    if let Some(i) = idx {
        Ok(u32::try_from(i)?)
    } else {
        Err(anyhow!(
            "Line {line_num}: Unknown Type 0 instruction: {}",
            opcode.as_str()
        ))
    }
}

fn parse_immediate(
    instr_caps: &Captures<'_>,
    label_to_addr: &HashMap<String, Address>,
    instr_type: usize,
    line_num: usize,
) -> Result<u32> {
    if let Some(immed) = instr_caps.name("immediate_val") {
        let Ok(raw_val) = immed.as_str().parse::<u32>() else {
            return Err(anyhow!(
                "Line {line_num}: Failed to parse immediate value: {}",
                immed.as_str()
            ));
        };

        if raw_val > MAX_IMMEDIATE_VAL {
            return Err(anyhow!(
                "Immediate exceeds maximum allowed value: {raw_val} > {MAX_IMMEDIATE_VAL}"
            ));
        }

        Ok(raw_val)
    } else if let Some(immed) = instr_caps.name("immediate_label") {
        if let Some(val) = label_to_addr.get(immed.as_str()) {
            Ok(*val)
        } else {
            return Err(anyhow!(
                "Line {line_num}: Undefined label {}",
                immed.as_str()
            ));
        }
    } else {
        return Err(anyhow!(
            "Line {line_num}: Parsing failiure. Invalid Type {instr_type} immediate argument"
        ));
    }
}

fn parse_reg(
    instr_caps: &Captures<'_>,
    instr_type: usize,
    reg_group: RegisterGroup,
    reg_arg_num: usize,
    line_num: usize,
) -> Result<usize> {
    if let Some(reg) = instr_caps.name(&format!("reg_{reg_arg_num}")) {
        let reg_prefix = match reg_group {
            RegisterGroup::General => ['r', 'R'],
            RegisterGroup::FloatingPoint => ['f', 'F'],
        };
        let Ok(parsed_reg) = reg.as_str().replacen(reg_prefix, "", 1).parse::<usize>() else {
            return Err(anyhow!(
                "Line {line_num}: Failed to parse register argument: {}",
                reg.as_str()
            ));
        };
        match reg_group {
            RegisterGroup::General => {
                if !(0..NUM_GEN_REGS).contains(&parsed_reg) {
                    return Err(anyhow!("Line {line_num}: Invalid register number {parsed_reg}. Valid range is [0-{NUM_GEN_REGS})"));
                }
            }
            RegisterGroup::FloatingPoint => {
                if !(0..NUM_FLOAT_REGS).contains(&parsed_reg) {
                    return Err(anyhow!("Line {line_num}: Invalid register number {parsed_reg}. Valid range is [0-{NUM_FLOAT_REGS})"));
                }
            }
        }

        Ok(parsed_reg)
    } else {
        Err(anyhow!(
            "Line {line_num}: Parsing failiure. Invalid Type {instr_type} register argument"
        ))
    }
}

fn parse_type_0(instr: &str, line_num: usize) -> Result<Instruction> {
    static TYPE_0_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(INSTR_TYPE_0_REGEX).unwrap());
    let Some(caps) = TYPE_0_REGEX.captures(instr) else {
        return Err(anyhow!(
            "Line {line_num}: Parsing failure. Invalid Type 0 instruction: {instr}"
        ));
    };

    let opcode = parse_opcode(instr, &caps, 0, line_num)?;
    Ok(Instruction::Type0 { opcode })
}

fn parse_type_1(
    instr: &str,
    label_to_addr: &HashMap<String, Address>,
    line_num: usize,
) -> Result<Instruction> {
    static TYPE_1_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(INSTR_TYPE_1_REGEX).unwrap());
    let Some(caps) = TYPE_1_REGEX.captures(instr) else {
        return Err(anyhow!(
            "Line {line_num}: Parsing failure. Invalid Type 1 instruction: {instr}"
        ));
    };

    let opcode = parse_opcode(instr, &caps, 1, line_num)?;
    let immediate = parse_immediate(&caps, label_to_addr, 1, line_num)?;

    Ok(Instruction::Type1 { opcode, immediate })
}

fn parse_type_2(instr: &str, line_num: usize) -> Result<Instruction> {
    static TYPE_2_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(INSTR_TYPE_2_REGEX).unwrap());
    let Some(caps) = TYPE_2_REGEX.captures(instr) else {
        return Err(anyhow!(
            "Line {line_num}: Parsing failure. Invalid Type 2 instruction: {instr}"
        ));
    };

    let opcode = parse_opcode(instr, &caps, 2, line_num)?;
    let reg_1 = parse_reg(&caps, 2, RegisterGroup::General, 1, line_num)?;
    let reg_2 = parse_reg(&caps, 2, RegisterGroup::General, 2, line_num)?;

    Ok(Instruction::Type2 {
        opcode,
        reg_1,
        reg_2,
    })
}

fn parse_type_3(instr: &str, line_num: usize) -> Result<Instruction> {
    static TYPE_3_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(INSTR_TYPE_3_REGEX).unwrap());
    let Some(caps) = TYPE_3_REGEX.captures(instr) else {
        return Err(anyhow!(
            "Line {line_num}: Parsing failure. Invalid Type 3 instruction: {instr}"
        ));
    };

    let opcode = parse_opcode(instr, &caps, 3, line_num)?;
    let freg_1 = parse_reg(&caps, 3, RegisterGroup::FloatingPoint, 1, line_num)?;
    let freg_2 = parse_reg(&caps, 3, RegisterGroup::FloatingPoint, 2, line_num)?;

    Ok(Instruction::Type3 {
        opcode,
        freg_1,
        freg_2,
    })
}

fn parse_type_4(
    instr: &str,
    label_to_addr: &HashMap<String, Address>,
    line_num: usize,
) -> Result<Instruction> {
    static TYPE_4_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(INSTR_TYPE_4_REGEX).unwrap());
    let Some(caps) = TYPE_4_REGEX.captures(instr) else {
        return Err(anyhow!(
            "Line {line_num}: Parsing failure. Invalid Type 4 instruction: {instr}"
        ));
    };

    let opcode = parse_opcode(instr, &caps, 4, line_num)?;
    let reg_1 = parse_reg(&caps, 4, RegisterGroup::General, 1, line_num)?;
    let immediate = parse_immediate(&caps, label_to_addr, 4, line_num)?;

    Ok(Instruction::Type4 {
        opcode,
        reg_1,
        immediate,
    })
}

fn parse_type_5(instr: &str, line_num: usize) -> Result<Instruction> {
    static TYPE_5_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(INSTR_TYPE_5_REGEX).unwrap());
    let Some(caps) = TYPE_5_REGEX.captures(instr) else {
        return Err(anyhow!(
            "Line {line_num}: Parsing failure. Invalid Type 5 instruction: {instr}"
        ));
    };

    let opcode = parse_opcode(instr, &caps, 5, line_num)?;
    let reg_1 = parse_reg(&caps, 5, RegisterGroup::General, 1, line_num)?;
    let reg_2 = parse_reg(&caps, 5, RegisterGroup::General, 2, line_num)?;
    let reg_3 = parse_reg(&caps, 5, RegisterGroup::General, 3, line_num)?;

    Ok(Instruction::Type5 {
        opcode,
        reg_1,
        reg_2,
        reg_3,
    })
}

fn parse_type_6(instr: &str, line_num: usize) -> Result<Instruction> {
    static TYPE_6_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(INSTR_TYPE_6_REGEX).unwrap());
    let Some(caps) = TYPE_6_REGEX.captures(instr) else {
        return Err(anyhow!(
            "Line {line_num}: Parsing failure. Invalid Type 6 instruction: {instr}"
        ));
    };

    let opcode = parse_opcode(instr, &caps, 6, line_num)?;
    let freg_1 = parse_reg(&caps, 6, RegisterGroup::FloatingPoint, 1, line_num)?;
    let freg_2 = parse_reg(&caps, 6, RegisterGroup::FloatingPoint, 2, line_num)?;
    let freg_3 = parse_reg(&caps, 6, RegisterGroup::FloatingPoint, 3, line_num)?;

    Ok(Instruction::Type6 {
        opcode,
        freg_1,
        freg_2,
        freg_3,
    })
}

fn parse_instruction(
    instr: &str,
    label_to_addr: &HashMap<String, Address>,
    line_num: usize,
) -> Result<Instruction> {
    let instr_type = get_instr_type(instr)?;

    match instr_type {
        0 => parse_type_0(instr, line_num),
        1 => parse_type_1(instr, label_to_addr, line_num),
        2 => parse_type_2(instr, line_num),
        3 => parse_type_3(instr, line_num),
        4 => parse_type_4(instr, label_to_addr, line_num),
        5 => parse_type_5(instr, line_num),
        6 => parse_type_6(instr, line_num),
        _ => Err(anyhow!("Invalid instruction type: {instr_type}")),
    }
}

fn get_bin_rep(instr: &Instruction) -> Result<[u8; 4]> {
    let translated = match instr {
        Instruction::Type0 { opcode } => {
            let mut raw = 0u32;
            raw |= opcode << TYPE_FIELD_WIDTH;
            raw
        }
        Instruction::Type1 { opcode, immediate } => {
            let mut raw = 1u32;
            raw |= opcode << TYPE_FIELD_WIDTH;
            raw |= immediate << (TYPE_FIELD_WIDTH + TYPE_1_OPCODE_FIELD_WIDTH);
            raw
        }
        Instruction::Type2 {
            opcode,
            reg_1,
            reg_2,
        } => {
            let mut raw = 2u32;
            raw |= opcode << TYPE_FIELD_WIDTH;
            raw |= u32::try_from(*reg_1)? << (TYPE_FIELD_WIDTH + TYPE_2_OPCODE_FIELD_WIDTH);
            raw |= u32::try_from(*reg_2)?
                << (TYPE_FIELD_WIDTH + TYPE_2_OPCODE_FIELD_WIDTH + REG_FIELD_WIDTH);
            raw
        }
        Instruction::Type3 {
            opcode,
            freg_1,
            freg_2,
        } => {
            let mut raw = 3u32;
            raw |= opcode << TYPE_FIELD_WIDTH;
            raw |= u32::try_from(*freg_1)? << (TYPE_FIELD_WIDTH + TYPE_3_OPCODE_FIELD_WIDTH);
            raw |= u32::try_from(*freg_2)?
                << (TYPE_FIELD_WIDTH + TYPE_3_OPCODE_FIELD_WIDTH + REG_FIELD_WIDTH);
            raw
        }
        Instruction::Type4 {
            opcode,
            reg_1,
            immediate,
        } => {
            let mut raw = 4u32;
            raw |= opcode << TYPE_FIELD_WIDTH;
            raw |= u32::try_from(*reg_1)? << (TYPE_FIELD_WIDTH + TYPE_4_OPCODE_FIELD_WIDTH);
            raw |= immediate << (TYPE_FIELD_WIDTH + TYPE_4_OPCODE_FIELD_WIDTH + REG_FIELD_WIDTH);
            raw
        }
        Instruction::Type5 {
            opcode,
            reg_1,
            reg_2,
            reg_3,
        } => {
            let mut raw = 5u32;
            raw |= opcode << TYPE_FIELD_WIDTH;
            raw |= u32::try_from(*reg_1)? << (TYPE_FIELD_WIDTH + TYPE_5_OPCODE_FIELD_WIDTH);
            raw |= u32::try_from(*reg_2)?
                << (TYPE_FIELD_WIDTH + TYPE_5_OPCODE_FIELD_WIDTH + REG_FIELD_WIDTH);
            raw |= u32::try_from(*reg_3)?
                << (TYPE_FIELD_WIDTH
                    + TYPE_5_OPCODE_FIELD_WIDTH
                    + REG_FIELD_WIDTH
                    + REG_FIELD_WIDTH);
            raw
        }
        Instruction::Type6 {
            opcode,
            freg_1,
            freg_2,
            freg_3,
        } => {
            let mut raw = 6u32;
            raw |= opcode << TYPE_FIELD_WIDTH;
            raw |= u32::try_from(*freg_1)? << (TYPE_FIELD_WIDTH + TYPE_6_OPCODE_FIELD_WIDTH);
            raw |= u32::try_from(*freg_2)?
                << (TYPE_FIELD_WIDTH + TYPE_6_OPCODE_FIELD_WIDTH + REG_FIELD_WIDTH);
            raw |= u32::try_from(*freg_3)?
                << (TYPE_FIELD_WIDTH
                    + TYPE_6_OPCODE_FIELD_WIDTH
                    + REG_FIELD_WIDTH
                    + REG_FIELD_WIDTH);
            raw
        }
    };

    Ok(translated.to_be_bytes())
}

fn get_instructions(
    conts: &str,
    label_to_addr: &HashMap<String, Address>,
    comment_lines: &mut HashSet<usize>,
) -> Result<Vec<Instruction>> {
    let mut instructions: Vec<Instruction> = Vec::new();

    let mut line_num = 1;
    for line in conts.lines() {
        while comment_lines.remove(&line_num) {
            line_num += 1;
        }
        let cleaned = line.trim().replace(':', "");
        // Only parse as instruction if it's not a label
        if !label_to_addr.contains_key(&cleaned) {
            instructions.push(parse_instruction(&cleaned, label_to_addr, line_num)?);
        }
        line_num += 1;
    }

    Ok(instructions)
}

fn write_program(instrs: &Vec<Instruction>, opts: &AssemblerOptions) -> Result<()> {
    // get the file open
    let output_path: PathBuf = if let Some(ref path) = opts.output_path {
        path.into()
    } else {
        DEFAULT_OUTPUT_PATH.into()
    };

    // get the bin rep for each instruction
    let mut bin_reps: Vec<u8> = Vec::new();

    for instr in instrs {
        bin_reps.append(&mut get_bin_rep(instr)?.into());
    }

    // write
    std::fs::write(output_path, &bin_reps)?;

    Ok(())
}

/// main driver function
fn assemble(opts: &AssemblerOptions) -> Result<()> {
    let file_conts = read_input(&opts.input_path)?;
    let (clean_conts, mut comment_lines) = strip_comments(&file_conts);

    // get symbol to address map
    let label_to_addr = get_label_to_addr_map(&clean_conts);
    // TODO: figure out the reverse map Chip was talking about for debugging purposes...
    // parse instructions... (maybe output a second file with debugging symbols")
    let instructions = get_instructions(&clean_conts, &label_to_addr, &mut comment_lines)?;
    write_program(&instructions, opts)?;

    Ok(())
}

fn main() {
    let args = AssemblerArgs::parse();
    let opts: AssemblerOptions = args.into();

    if let Err(e) = assemble(&opts) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
