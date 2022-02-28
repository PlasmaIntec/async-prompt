use std::ops::DerefMut;
use std::sync::{Mutex};

pub struct LineBuffer {
    pub string: Mutex<String>
}

impl LineBuffer {
    pub fn get_string(&self) -> String {
        self.string.lock().unwrap().deref_mut().to_string()
    }

    pub fn replace_string(&mut self, new_string: String) {
        let mut string = self.string.lock().unwrap();
        *string = new_string;
    }

    pub fn append_string(&mut self, new_string: String) {
        let mut string = self.string.lock().unwrap();
        string.push_str(&new_string);
    }
}
