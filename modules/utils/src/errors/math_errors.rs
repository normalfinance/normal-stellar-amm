use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum MathError {
    #[doc = "MathError: NumberOverflow"]
    NumberOverflow = 510,
    #[doc = "MathError: Generic math error"]
    MathError = 511,
    #[doc = "MathError: Addition operation caused overflow"]
    AdditionOverflow = 512,
    #[doc = "MathError: Subtraction operation caused underflow"]
    SubtractionUnderflow = 513,
    #[doc = "MathError: Multiplication operation caused overflow"]
    MultiplicationOverflow = 514,
    #[doc = "MathError: Division by zero"]
    DivisionByZero = 515,
}
