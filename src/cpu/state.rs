use crate::{
    cpu::{
        ArithmeticKind, CbType, CpuFault, Destination, PushOperand, Register, Register16,
        UnaryOperation,
    },
    memory::Interrupt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InterruptSelection {
    Selected(Interrupt),
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReturnKind {
    Normal,
    Interrupt,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ExecutionState {
    FetchOpcode {
        skip_increment: bool,
    },
    ReadDisplacement {
        taken: bool,
    },
    ApplyJump {
        target: u16,
    },
    ReadImmediate8 {
        target: Destination,
    },
    ReadImmediate16Low {
        destination: Register16,
    },
    ReadImmediate16High {
        destination: Register16,
        low: u8,
    },
    ReadJumpLow {
        taken: bool,
    },
    ReadJumpHigh {
        taken: bool,
        low: u8,
    },
    ReadMemory {
        addr: u16,
        destination: Register,
    },
    ReadMemoryAndUpdateHL {
        addr: u16,
        operation: UnaryOperation,
    },
    ReadOperation {
        addr: u16,
        operation: UnaryOperation,
    },
    ReadImmediateAccumulator {
        value: u8,
        operation: ArithmeticKind,
    },
    ReadMemoryAccumulator {
        addr: u16,
        value: u8,
        operation: ArithmeticKind,
    },
    ReadHighMemoryImmediate {
        store: Option<u8>,
    },
    WriteMemory {
        addr: u16,
        value: u8,
    },
    WriteMemoryAndUpdateHL {
        addr: u16,
        value: u8,
        operation: UnaryOperation,
    },
    Push {
        addr: u16,
        value: u16,
    },
    PushHigh {
        addr: u16,
        high: u8,
        low: u8,
    },
    PushLow {
        addr: u16,
        low: u8,
    },
    PopLow {
        addr: u16,
        target: PushOperand,
    },
    PopHigh {
        addr: u16,
        low: u8,
        target: PushOperand,
    },
    ReadCallLow {
        taken: bool,
        stack_addr: u16,
    },
    ReadCallHigh {
        taken: bool,
        low: u8,
        stack_addr: u16,
    },
    Call {
        jump_addr: u16,
        stack_addr: u16,
        return_addr_high: u8,
        return_addr_low: u8,
    },
    PushCallerHigh {
        jump_addr: u16,
        stack_addr: u16,
        return_addr_high: u8,
        return_addr_low: u8,
    },
    PushCallerLow {
        jump_addr: u16,
        stack_addr: u16,
        return_addr_low: u8,
    },
    ReadReturnLow {
        addr: u16,
        kind: ReturnKind,
    },
    ReadReturnHigh {
        addr: u16,
        low: u8,
        kind: ReturnKind,
    },
    FinishReturn {
        target: u16,
        kind: ReturnKind,
    },
    ReturnConditional {
        taken: bool,
        addr: u16,
    },
    Unary16 {
        target: Register16,
        value: u16,
    },
    Addition16 {
        lhs: u16,
        rhs: u16,
    },
    ReadSpOffset {
        sp: u16,
        hl: bool,
    },
    DelaySpOffset {
        sp: u16,
        offset: i8,
        hl: bool,
    },
    ApplySpOffset {
        sp: u16,
        offset: i8,
        hl: bool,
    },
    ReadAbsoluteLow,
    ReadAbsoluteHigh {
        low: u8,
    },
    StoreAToAbsoluteLow {
        value: u8,
    },
    StoreAToAbsoluteHigh {
        value: u8,
        low: u8,
    },
    TransferHLtoSP {
        hl: u16,
    },
    ReadLowToStoreSp {
        sp: u16,
    },
    ReadHighToStoreSp {
        sp: u16,
        low: u8,
    },
    WriteSpLow {
        sp_low: u8,
        sp_high: u8,
        addr: u16,
    },
    DecodeCbOpcode,
    ReadHLForCbOpcode {
        addr: u16,
        cb_type: CbType,
    },
    InterruptEntry {
        pc: u16,
        sp: u16,
    },
    InterruptWait {
        pc: u16,
        sp: u16,
    },
    InterruptPushHigh {
        pc_high: u8,
        pc_low: u8,
        sp: u16,
    },
    InterruptPushLow {
        pc_low: u8,
        sp: u16,
        captured_enable: u8,
    },
    InterruptJump {
        interrupt: InterruptSelection,
    },
    Halted,
    Stopped,
    Faulted(CpuFault),
}

#[derive(Debug, Clone, Copy)]
pub(super) enum HaltBehavior {
    Sleep,
    SuppressNextIncrement,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum StopBehavior {
    Stop { skip_padding: bool },
    Halt,
    Continue,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CycleTransition {
    Continue(ExecutionState),
    InstructionFinished,
    InterruptEntryFinished { interrupt: Interrupt },
    InterruptSelectionResolved { interrupt: InterruptSelection },
    InterruptEntryCancelled,
    HaltFinished(HaltBehavior),
    StopFinished(StopBehavior),
    Faulted(CpuFault),
}
