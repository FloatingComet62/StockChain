pub struct Communication {
    pub message: String,
}

impl Communication {
    pub fn new(message: String) -> Self {
        Communication { message }
    }
}