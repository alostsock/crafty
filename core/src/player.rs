use std::fmt;

pub struct Player {
    pub job_level: u32,
    pub craftsmanship: u32,
    pub control: u32,
    pub cp: u32,
}

impl Player {
    pub fn new(job_level: u32, craftsmanship: u32, control: u32, cp: u32) -> Self {
        Player {
            job_level,
            craftsmanship,
            control,
            cp,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "lv{:>2} / {} craftsmanship / {} control / {} cp",
            self.job_level, self.craftsmanship, self.control, self.cp
        )
    }
}
