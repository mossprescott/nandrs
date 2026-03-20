/// Alternate Hack CPU implementation, attempting to dispatch 2 Hack instructions per cycle.
///
/// Observation: a common pattern in Hack programs is to load a value/address using a sequence like
/// "@1234; D=A" or "@R5; D=M". In any such sequence, the first instruction only needs the
/// instruction word and access to the A register, while the second instruction might access the
/// memory, branch, etc.
///
/// Statistically, about 30% of all instructions in ROM are "A"-instructions (see Pong.asm).
/// Presumably they make up something like 30% of instructions *executed* as well. This architecture
/// will make all those instructions consume 0 cycles.
///
/// What if the CPU could handle *both* a load to A *and* the ensuing instruction in a single cycle?
/// That would be a fun way to spend money on hardware, while maintaining compatibitly with the vast
/// and valuable library of Hack softwarez.
///
/// To execute 2 instructions, we need to feed 2 instructions into the CPU on each cycle. Since we
/// don't have a dual-ported or double-clocked ROM in this project, we'll just fake it by wiring up
/// a second ROM which we'll load with the same binary.

use assignments::project_02::Inc16;
use assignments::project_05::{self, Decode, Project05Component};
use simulator::{self, AsConst, Component, IC, Input, Input16, Output, Output16, Reflect, Chip, expand};
use simulator::component::{Computational16, ROM16, MemorySystem16};
use simulator::nat::N16;
use simulator::simulate::{ChipState, BusResident, ROMHandle};

/// CPU which (potentially) decodes and executes a pair of instructions in each cycle.
#[derive(Reflect, Chip)]
pub struct CPU {
    /// Return to a known state (i.e. jump to address 0)
    pub reset: Input,

    /// Address of the next instruction to load
    pub pc: Output16,

    /// The bits of the current instruction
    pub instr0: Input16,

    /// The bits of the instruction following the current instruction
    pub instr1: Input16,

    pub mem_data_out: Output16,
    pub mem_write: Output,

    pub mem_addr: Output16,

    pub mem_data_in: Input16,
}

impl Component for CPU {
    type Target = DoubleComponent;

    expand! { |this| {
        decode0: Decode {
            instr: this.instr0.into(),
            is_c: Output::new(), is_a: Output::new(),
            read_m: Output::new(),
            zx: Output::new(), nx: Output::new(),
            zy: Output::new(), ny: Output::new(),
            f: Output::new(), no: Output::new(),
            write_a: Output::new(), write_m: Output::new(), write_d: Output::new(),
            jmp_lt: Output::new(), jmp_eq: Output::new(), jmp_gt: Output::new(),
        },

        decode1: Decode {
            instr: this.instr1.into(),
            is_c: Output::new(), is_a: Output::new(),
            read_m: Output::new(),
            zx: Output::new(), nx: Output::new(),
            zy: Output::new(), ny: Output::new(),
            f: Output::new(), no: Output::new(),
            write_a: Output::new(), write_m: Output::new(), write_d: Output::new(),
            jmp_lt: Output::new(), jmp_eq: Output::new(), jmp_gt: Output::new(),
        },


    }}
}


#[derive(Reflect, Chip)]
pub struct Computer {
    /// A way to force the CPU to return to a known state (i.e. jump to address 0)
    pub reset: Input,

    /// Useful for debugging, but also acts as a root for traversing the graph
    pub pc: Output16,
}

impl Component for Computer {
    type Target = DoubleComponent;

    expand! { |this| {
        mem_out: forward Output16::new(),

        rom0: ROM16 {
            size: 32 * 1024,
            addr: this.pc.into(),
            out:  Output16::new(),
        },

        next_pc: Inc16 {
            a: this.pc.into(),
            out: Output16::new(),
        },
        rom1: ROM16 {
            size: 32 * 1024,
            addr: next_pc.out.into(),
            out:  Output16::new(),
        },

        cpu: CPU {
            reset:        this.reset,
            pc:           this.pc,
            instr0:       rom0.out.into(),
            instr1:       rom1.out.into(),
            mem_data_out: Output16::new(),
            mem_write:    Output::new(),
            mem_addr:     Output16::new(),
            mem_data_in:  mem_out.into(),
        },

        memory: MemorySystem16 {
            addr:     cpu.mem_addr.into(),
            write:    cpu.mem_write.into(),
            data_in:  cpu.mem_data_out.into(),
            data_out: mem_out,
        },
    }}
}


pub enum DoubleComponent {
    Project05(Project05Component),
    CPU(CPU),
    Computer(Computer),
}

impl<C: Into<Project05Component>> From<C> for DoubleComponent {
    fn from(c: C) -> Self {
        DoubleComponent::Project05(c.into())
    }
}
impl From<CPU>      for DoubleComponent { fn from(c: CPU)      -> Self { DoubleComponent::CPU(c)      } }
impl From<Computer> for DoubleComponent { fn from(c: Computer) -> Self { DoubleComponent::Computer(c) } }

impl Component for DoubleComponent {
    type Target = DoubleComponent;

    fn expand(&self) -> Option<IC<DoubleComponent>> {
        match self {
            DoubleComponent::Project05(c) => c.expand().map(|ic| IC { name: ic.name, intf: ic.intf, components: ic.components.into_iter().map(Into::into).collect() }),
            DoubleComponent::CPU(c)       => c.expand(),
            DoubleComponent::Computer(c)  => c.expand(),
        }
    }
}

impl Reflect for DoubleComponent {
    fn reflect(&self) -> simulator::Interface {
        match self {
            DoubleComponent::Project05(c) => c.reflect(),
            DoubleComponent::CPU(c)       => c.reflect(),
            DoubleComponent::Computer(c)  => c.reflect(),
        }
    }
    fn name(&self) -> String {
        match self {
            DoubleComponent::Project05(c) => c.name(),
            DoubleComponent::CPU(c)       => c.name(),
            DoubleComponent::Computer(c)  => c.name(),
        }
    }
}

/// Find the two ROMs (rom0 at pc, rom1 at pc+1) in the chip state.
pub fn find_roms(state: &ChipState<N16, N16>) -> (ROMHandle<N16, N16>, ROMHandle<N16, N16>) {
    let roms: Vec<_> = state.bus_residents().iter()
        .filter_map(|r| if let BusResident::ROM(h) = r { Some(h.clone()) } else { None })
        .collect();
    assert_eq!(roms.len(), 2, "expected 2 ROMs, found {}", roms.len());
    (roms[0].clone(), roms[1].clone())
}

/// Recursively expand until only Nands, Registers, RAMs, ROMs, and MemorySystems are left.
pub fn flatten<C: Reflect + Into<DoubleComponent>>(chip: C) -> IC<Computational16> {
    fn go(comp: DoubleComponent) -> Vec<Computational16> {
        match comp.expand() {
            None => match comp {
                DoubleComponent::Project05(p) =>
                    project_05::flatten(p)
                        .components,
                _ => panic!("Did not reduce to primitive: {:?}", comp.name()),
            },
            Some(ic) => ic.components.into_iter().flat_map(go).collect(),
        }
    }
    IC {
        name: format!("{} (flat)", chip.name()),
        intf: chip.reflect(),
        components: go(chip.into()),
    }
}

impl AsConst for DoubleComponent {
    fn as_const(&self) -> Option<u64> {
        if let DoubleComponent::Project05(c) = self { c.as_const() } else { None }
    }
}

#[cfg(test)]
mod test {
    use assignments::tests::test_05;
    use simulator::Chip;
    use simulator::component::Computational;
    use simulator::print_graph;

    use crate::computer::{Computer, flatten};

    #[test]
    fn computer_max_behavior() {

        let chip = Computer::chip();

        // When it breaks, it's nice to see what it tried to do
        print!("{}", print_graph(&chip));

        test_05::test_computer_max_behavior(flatten(chip));
    }

    #[test]
    fn computer_optimal() {
        let components = flatten(Computer::chip()).components;
        let memsys = components.iter().filter(|c| matches!(c, Computational::MemorySystem(_))).count();
        let roms   = components.iter().filter(|c| matches!(c, Computational::ROM(_))).count();
        let nands  = components.iter().filter(|c| matches!(c, Computational::Nand(_))).count();
        let adders = components.iter().filter(|c| matches!(c, Computational::Adder(_))).count();
        let muxes  = components.iter().filter(|c| matches!(c, Computational::Mux(_))).count();
        assert_eq!(memsys,  1);
        assert_eq!(roms,    2);    // Compare to 1
        assert_eq!(nands, 171);    // Compare to 166
        assert_eq!(adders, 62);    // Compare to 31
        assert_eq!(muxes,  16);    // Compare to 15
    }
}