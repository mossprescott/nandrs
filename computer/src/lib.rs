pub mod disasm;
pub mod display;
pub mod keyboard;

use assignments::project_03::Project03Component;
use assignments::project_05::Project05Component;
use simulator::{IC, Reflect, Component as _};

/// Recursively expand high-level components (projects 3 and 5), until only primitives and simple
/// logic are left (projects 1 and 2).
pub fn half_flatten<C: Reflect + Into<Project05Component>>(chip: C) -> IC<Project05Component> {
    fn go(comp: Project05Component) -> Vec<Project05Component> {
        // Stop at Project02: don't expand ALU, adders, etc. into Nands.
        if let Project05Component::Project03(Project03Component::Project02(_)) = &comp {
            vec![comp]
        }
        else {
            match comp.expand() {
                None => vec![comp],
                Some(ic) => ic.components.into_iter().flat_map(go).collect(),
            }
        }
    }
    IC {
        name: format!("{} (half-flat)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
}

pub fn fmt_commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(','); }
        out.push(c);
    }
    out.chars().rev().collect()
}
