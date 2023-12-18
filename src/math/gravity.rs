use core::fmt::Debug;

pub struct Gravity {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Debug for Gravity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Gravity: ({:06.2}, {:06.2}, {:06.2})", self.x, self.y, self.z)
    }
}
