use core::fmt::Debug;

pub struct Gravity {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Debug for Gravity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gyro")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("z", &self.z)
            .finish()
    }
}
