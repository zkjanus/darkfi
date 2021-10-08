use std::collections::HashMap;
use std::fs::File;
use std::io;

use crate::state::{Constants, Line};

#[derive(Default)]
pub struct Syntax {
    pub contracts: HashMap<String, Vec<Line>>,
    pub circuits: HashMap<String, Vec<Line>>,
    pub constants: Constants,
}

impl Syntax {
    pub fn new() -> Self {
        Syntax {
            contracts: HashMap::new(),
            circuits: HashMap::new(),
            constants: Constants::new(),
        }
    }

    fn parse_contract(&mut self, line: Line, iter: &mut std::slice::Iter<'_, Line>) {
        assert!(line.tokens[0] == "contract");

        if line.tokens.len() != 3 || line.tokens[2] != "{" {
            panic!("malformed contract opening\n{:#?}", line);
        }

        let name = line.tokens[1].clone();
        if self.contracts.contains_key(name.as_str()) {
            panic!("duplicate contract {}\n:{:#?}", name, line);
        }

        let mut lines: Vec<Line> = vec![];

        loop {
            let l = iter.next();
            if l.is_none() {
                panic!("premature eof while parsing {} contract\n{:#?}", name, line);
            }
            let l = l.unwrap();

            assert!(!l.tokens.is_empty());
            if l.tokens[0] == "}" {
                break;
            }

            lines.push(l.clone());
        }

        self.contracts.insert(name, lines);
    }

    fn parse_circuit(&mut self, line: Line, iter: &mut std::slice::Iter<'_, Line>) {
        assert!(line.tokens[0] == "circuit");

        if line.tokens.len() != 3 || line.tokens[2] != "{" {
            panic!("malformed circuit opening\n{:#?}", line);
        }

        let name = line.tokens[1].clone();
        if self.circuits.contains_key(name.as_str()) {
            panic!("duplicate circuit {}\n{:#?}", name, line);
        }

        let mut lines: Vec<Line> = vec![];

        loop {
            let l = iter.next();
            if l.is_none() {
                panic!("premature eof while parsing {} circiuit\n{:#?}", name, line);
            }

            let l = l.unwrap();

            assert!(!l.tokens.is_empty());
            if l.tokens[0] == "}" {
                break;
            }

            lines.push(l.clone());
        }

        self.circuits.insert(name, lines);
    }

    fn parse_constant(&mut self, line: Line) {
        assert!(line.tokens[0] == "constant");
    }
}

pub fn load_lines(lines: io::Lines<io::BufReader<File>>) -> Vec<Line> {
    let mut n: u32 = 0;
    let mut source = vec![];

    for original_line in lines {
        if let Ok(ol) = original_line {
            let line_number = n + 1;
            n += 1;

            // Remove whitespace on both sides
            let orig = ol.clone();
            let line = orig.trim_start().trim_end();
            // Strip out comments
            let spl: Vec<&str> = line.split('#').collect();
            let line = spl[0];
            if line.is_empty() {
                continue;
            }
            // Split at whitespace
            let spl: Vec<String> = line.split(' ').map(|s| s.to_string()).collect();

            // TODO: Line number should represent the one in the file
            //let line_number = n + 1;
            // n += 1;
            //source.push(Line::new(spl, ol, line_number));

            source.push(Line::new(
                spl,
                orig.trim_start().trim_end().to_string(),
                line_number,
            ));
        }
    }

    source
}

pub fn parse_lines(lines: Vec<Line>) -> Syntax {
    let mut syntax = Syntax::new();
    let mut iter = lines.iter();

    loop {
        let line = iter.next();
        if line.is_none() {
            break;
        }

        let line = line.unwrap();
        assert!(!line.tokens.is_empty());

        match line.tokens[0].as_str() {
            "contract" => syntax.parse_contract(line.clone(), &mut iter),
            "circuit" => syntax.parse_circuit(line.clone(), &mut iter),
            "constant" => syntax.parse_constant(line.clone()),
            "}" => panic!("unmatched delimiter '}}'\n{:#?}", line),
            _ => unreachable!(),
        }
    }

    syntax
}
