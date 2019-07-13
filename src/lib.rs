use std::str::FromStr;

pub struct Args<'a> {
    args: &'a Vec<String>,
}

impl<'a> Args<'a> {
    pub fn new(args: &'a Vec<String>) -> Args {
        Args { args }
    }
    pub fn get_str(&self, index: usize, def: &'a str) -> &'a str {
        match self.args.get(index) {
            Some(ret) => ret,
            None => def,
        }
    }
    pub fn get<T: FromStr>(&self, index: usize, def: T) -> T {
        match self.args.get(index) {
            Some(ret) => match ret.parse::<T>() {
                Ok(ret) => ret,
                Err(_) => def, // or panic
            },
            None => def,
        }
    }
}


