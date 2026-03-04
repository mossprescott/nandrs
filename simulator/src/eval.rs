/// A simple evaluator for chips that reduce to a collection of Nands.
///
/// There is no clock and no state.

use std::collections::HashMap;
use std::rc::Rc;

use crate::declare::Component;

/// Evaluate a component given named input values, returning named output values.
///
/// Uses `reflect()` to identify wires by identity and `expand()` to decompose compound
/// components. A component that returns `None` from `expand()` is the NAND primitive.
pub fn eval<'a, C, I>(chip: &C, inputs: I) -> HashMap<String, bool>
where
    C: Component,
    C::Target: Component<Target = C::Target>,
    I: IntoIterator<Item = (&'a str, bool)>,
{
    let intf = chip.reflect();

    // Map wire identity (Rc pointer) → current value.
    let mut wire_state: HashMap<usize, bool> = HashMap::new();

    // Seed with the provided input values.
    for (name, value) in inputs {
        if let Some(busref) = intf.inputs.get(name) {
            wire_state.insert(wire_id(&busref.id), value);
        }
    }

    eval_component(chip, &mut wire_state);

    // Read named outputs.
    intf.outputs
        .iter()
        .map(|(name, busref)| (name.clone(), wire_state.get(&wire_id(&busref.id)).copied().unwrap_or(false)))
        .collect()
}

fn wire_id(rc: &Rc<()>) -> usize {
    Rc::as_ptr(rc) as usize
}

fn eval_component<C>(component: &C, wire_state: &mut HashMap<usize, bool>)
where
    C: Component,
    C::Target: Component<Target = C::Target>,
{
    let intf = component.reflect();

    match component.expand() {
        None => {
            // Primitive NAND: out = !(a & b)
            let a = wire_state.get(&wire_id(&intf.inputs["a"].id)).copied().unwrap_or(false);
            let b = wire_state.get(&wire_id(&intf.inputs["b"].id)).copied().unwrap_or(false);
            wire_state.insert(wire_id(&intf.outputs["out"].id), !(a & b));
        }
        Some(sub_components) => {
            // Hmm. Need to sort here? Will find out eventually
            for sub in &sub_components {
                eval_component(sub, wire_state);
            }
        }
    }
}
