use diagweave::set;

set! {
    pub SetA = {
        #[display("shared string: {0}")]
        Shared(String),
    }

    pub SetB = {
        #[display("shared int: {0}")]
        Shared(i32),
    }

    // Attempting to combine SetA and SetB where Shared variant has conflicting field types (String vs i32)
    pub SetC = SetA | SetB
}

fn main() {}
