use bitflags::bitflags;
// Flags taken linux kernel include/linux/ioport.h

const BUS_BITS: u64 = 0x000000ff; /* Bus-specific bits */

const TYPE_BITS: u64 = 0x00001f00; /* Resource type */
const IO: u64 = 0x00000100; /* PCI/ISA I/O ports */
const MEM: u64 = 0x00000200;
const REG: u64 = 0x00000300; /* Register offsets */
const IRQ: u64 = 0x00000400;
const DMA: u64 = 0x00000800;
const BUS: u64 = 0x00001000;

const PREFETCH: u64 = 0x00002000; /* No side effects */
const READONLY: u64 = 0x00004000;
const CACHEABLE: u64 = 0x00008000;
const RANGELENGTH: u64 = 0x00010000;
const SHADOWABLE: u64 = 0x00020000;

const SIZEALIGN: u64 = 0x00040000; /* size indicates alignment */
const STARTALIGN: u64 = 0x00080000; /* start field is alignment */

const MEM_64: u64 = 0x00100000;
const WINDOW: u64 = 0x00200000; /* forwarded by bridge */
const MUXED: u64 = 0x00400000; /* Resource is software muxed */

const EXT_TYPE_BITS: u64 = 0x01000000; /* Resource extended types */
const SYSRAM: u64 = 0x01000000; /* System RAM (modifier) */

/* SYSRAM specific bits. */
const SYSRAM_DRIVER_MANAGED: u64 = 0x02000000; /* Always detected via a driver. */
const SYSRAM_MERGEABLE: u64 = 0x04000000; /* Resource can be merged. */

const EXCLUSIVE: u64 = 0x08000000; /* Userland may not map this resource */

const DISABLED: u64 = 0x10000000;
const UNSET: u64 = 0x20000000; /* No address assigned yet */
const AUTO: u64 = 0x40000000;
const BUSY: u64 = 0x80000000; /* Driver has marked this resource busy */

/* I/O resource extended types */
const SYSTEM_RAM: u64 = (MEM | SYSRAM);

/* PnP IRQ specific bits (BITS) */
const IRQ_HIGHEDGE: u64 = (1 << 0);
const IRQ_LOWEDGE: u64 = (1 << 1);
const IRQ_HIGHLEVEL: u64 = (1 << 2);
const IRQ_LOWLEVEL: u64 = (1 << 3);
const IRQ_SHAREABLE: u64 = (1 << 4);
const IRQ_OPTIONAL: u64 = (1 << 5);
const IRQ_WAKECAPABLE: u64 = (1 << 6);

/* PnP DMA specific bits (BITS) */
const DMA_TYPE_MASK: u64 = 3;
const DMA_8BIT: u64 = 0;
const DMA_8AND16BIT: u64 = (1 << 0);
const DMA_16BIT: u64 = 2;

const DMA_MASTER: u64 = (1 << 2);
const DMA_BYTE: u64 = (1 << 3);
const DMA_WORD: u64 = (1 << 4);

const DMA_SPEED_MASK: u64 = (3 << 6);
const DMA_COMPATIBLE: u64 = (0 << 6);
const DMA_TYPEA: u64 = (1 << 6);
const DMA_TYPEB: u64 = (2 << 6);
const DMA_TYPEF: u64 = (3 << 6);

/* PnP memory I/O specific bits (BITS) */
const MEM_WRITEABLE: u64 = (1 << 0); /* dup: READONLY */
const MEM_CACHEABLE: u64 = (1 << 1); /* dup: CACHEABLE */
const MEM_RANGELENGTH: u64 = (1 << 2); /* dup: RANGELENGTH */
const MEM_TYPE_MASK: u64 = (3 << 3);
const MEM_8BIT: u64 = (0 << 3);
const MEM_16BIT: u64 = (1 << 3);
const MEM_8AND16BIT: u64 = (2 << 3);
const MEM_32BIT: u64 = (3 << 3);
const MEM_SHADOWABLE: u64 = (1 << 5); /* dup: SHADOWABLE */
const MEM_EXPANSIONROM: u64 = (1 << 6);
const MEM_NONPOSTED: u64 = (1 << 7);

/* PnP I/O specific bits (BITS) */
const IO_16BIT_ADDR: u64 = (1 << 0);
const IO_FIXED: u64 = (1 << 1);
const IO_SPARSE: u64 = (1 << 2);

/* PCI ROM control bits (BITS) */
const ROM_ENABLE: u64 = (1 << 0); /* ROM is enabled, same as PCI_ROM_ADDRESS_ENABLE */
const ROM_SHADOW: u64 = (1 << 1); /* Use RAM image, not ROM BAR */

/* PCI control bits.  Shares BITS with above PCI ROM.  */
const PCI_FIXED: u64 = (1 << 4); /* Do not move resource */
const PCI_EA_BEI: u64 = (1 << 5); /* BAR Equivalent Indicator */
bitflags! {
    pub struct PciResourceFlags: u64 {
        /// Bus-specific bits
        const BUS_BITS = BUS_BITS;
        /// Resource type
        const TYPE_BITS = TYPE_BITS;
        /// PCI/ISA I/O ports
        const IO = IO;
        const MEM = MEM;
        /// Register offsets
        const REG = REG;
        const IRQ = IRQ;
        const DMA = DMA;
        const BUS = BUS;
        /// No side effects
        const PREFETCH = PREFETCH;
        const READONLY = READONLY;
        const CACHEABLE = CACHEABLE;
        const RANGELENGTH = RANGELENGTH;
        /// dup: SHADOWABLE
        const SHADOWABLE = SHADOWABLE;
        /// Size indicates alignment
        const SIZEALIGN = SIZEALIGN;
        /// Start indicates alignment
        const STARTALIGN = STARTALIGN;
        const MEM_64 = MEM_64;
        /// Forwarded by bridge
        const WINDOW = WINDOW;
        /// Resource is software muxed
        const MUXED = MUXED;
        /// Resource extended types */
        const EXT_TYPE_BITS = EXT_TYPE_BITS;

        /// System RAM (modifier)
        const SYSRAM = SYSRAM;
        /// Always detected via a driver
        const SYSRAM_DRIVER_MANAGED = SYSRAM_DRIVER_MANAGED;
        /// Resource can be merged.
        const SYSRAM_MERGEABLE = SYSRAM_MERGEABLE;
        /// Userland may not map this resource
        const EXCLUSIVE = EXCLUSIVE;
        const DISABLED = DISABLED;
        /// No address assigned yet
        const UNSET = UNSET;
        const AUTO = AUTO;
        /// Driver has marked this resource busy.
        const BUSY = BUSY;
        const SYSTEM_RAM = SYSTEM_RAM;
        const IRQ_HIGHEDGE = IRQ_HIGHEDGE;
        const IRQ_LOWEDGE = IRQ_LOWEDGE;
        const IRQ_HIGHLEVEL = IRQ_HIGHLEVEL;
        const IRQ_LOWLEVEL = IRQ_LOWLEVEL;
        const IRQ_SHAREABLE = IRQ_SHAREABLE;
        const IRQ_OPTIONAL = IRQ_OPTIONAL;
        const IRQ_WAKECAPABLE = IRQ_WAKECAPABLE;
        const DMA_TYPE_MASK = DMA_TYPE_MASK;
        const DMA_8BIT = DMA_8BIT;
        const DMA_8AND16BIT = DMA_8AND16BIT;
        const DMA_16BIT = DMA_16BIT;
        const DMA_MASTER = DMA_MASTER;
        const DMA_BYTE = DMA_BYTE;
        const DMA_WORD = DMA_WORD;
        const DMA_SPEED_MASK = DMA_SPEED_MASK;
        const DMA_COMPATIBLE = DMA_COMPATIBLE;
        const DMA_TYPEA = DMA_TYPEA;
        const DMA_TYPEB = DMA_TYPEB;
        const DMA_TYPEF = DMA_TYPEF;
        const MEM_WRITEABLE = MEM_WRITEABLE;
        const MEM_CACHEABLE = MEM_CACHEABLE;
        const MEM_RANGELENGTH = MEM_RANGELENGTH;
        const MEM_TYPE_MASK = MEM_TYPE_MASK;
        const MEM_8BIT = MEM_8BIT;
        const MEM_16BIT = MEM_16BIT;
        const MEM_8AND16BIT = MEM_8AND16BIT;
        const MEM_32BIT = MEM_32BIT;
        const MEM_SHADOWABLE = MEM_SHADOWABLE;
        const MEM_EXPANSIONROM = MEM_EXPANSIONROM;
        const MEM_NONPOSTED = MEM_NONPOSTED;
        const IO_16BIT_ADDR = IO_16BIT_ADDR;
        const IO_FIXED = IO_FIXED;
        const IO_SPARSE = IO_SPARSE;
        /// ROM is enabled, same as PCI_ROM_ADDRESS_ENABLE.
        const ROM_ENABLE = ROM_ENABLE;
        const ROM_SHADOW = ROM_SHADOW;
        /// Do not move resource.
        const PCI_FIXED = PCI_FIXED;
        /// Use RAM image, not ROM BAR.
        const PCI_EA_BEI = PCI_EA_BEI;
        // All other bits may be set
        const _ = !0;
    }
}
