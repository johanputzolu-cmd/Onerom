// Copyright (C) 2025 Piers Finlayson <piers@piers.rocks>
//
// MIT License

/// MCU family
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// STM32F4 series
    Stm32f4,
    /// Raspberry Pi RP2350
    Rp2350,
}

impl core::fmt::Display for Family {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Family::Stm32f4 => write!(f, "STM32F4"),
            Family::Rp2350 => write!(f, "RP2350"),
        }
    }
}

/// GPIO Port designation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Port {
    /// No port (unused)
    None,
    /// Port 0 (RP2350)
    Zero,
    /// Port A (STM32)
    A,
    /// Port B (STM32)
    B,
    /// Port C (STM32)
    C,
    /// Port D (STM32)
    D,
}

impl core::fmt::Display for Port {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Port::None => write!(f, "PORT_NONE"),
            Port::Zero => write!(f, "PORT_0"),
            Port::A => write!(f, "PORT_A"),
            Port::B => write!(f, "PORT_B"),
            Port::C => write!(f, "PORT_C"),
            Port::D => write!(f, "PORT_D"),
        }
    }
}