use core::fmt::Debug;

use quaternion_core::{Vector3, Quaternion, RotationType, RotationSequence, to_euler_angles};

pub struct Rotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
impl Rotation {
    pub(crate) fn to_vector3(&self) -> Vector3<f32> {
        let q: Quaternion<f32> = (self.w, [self.x, self.y, self.z]);
        to_euler_angles(RotationType::Intrinsic, RotationSequence::XYZ, q)
    }
}

impl Debug for Rotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rotation: ({:03.2}, {:03.2}, {:03.2}, w:{:03.2})",
            self.x, self.y, self.z, self.w
        )
    }
}
