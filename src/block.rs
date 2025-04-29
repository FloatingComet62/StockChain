pub struct Block {
    pub index: u64,
    pub previous_hash: String,
    pub timestamp: u64,
    pub data: String,
    pub hash: String,
}

impl Block {
    pub fn new(index: u64, previous_hash: String, timestamp: u64, data: String, hash: String) -> Self {
        Block {
            index,
            previous_hash,
            timestamp,
            data,
            hash,
        }
    }

    pub fn verify_hash(&self) -> bool {
        // In a real blockchain, this would involve hashing the block's contents
        // and comparing it to the stored hash. Here, we'll just simulate that.
        let calculated_hash = format!("{}-{}-{}-{}", self.index, self.previous_hash, self.timestamp, self.data);
        self.hash == calculated_hash
    }

    pub fn to_string(&self) -> String {
        format!(
            "Block {{ index: {}, previous_hash: {}, timestamp: {}, data: {}, hash: {} }}",
            self.index, self.previous_hash, self.timestamp, self.data, self.hash
        )
    }
}