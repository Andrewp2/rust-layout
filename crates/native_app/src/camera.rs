use operad::UiPoint;

pub(crate) const CAMERA_NEAR_PLANE: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Camera3d {
    pub(crate) position: Vec3,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) speed: f32,
    pub(crate) fov_y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CameraBasis {
    pub(crate) forward: Vec3,
    pub(crate) right: Vec3,
    pub(crate) up: Vec3,
}

impl Default for Camera3d {
    fn default() -> Self {
        Self {
            position: Vec3::new(-8_000.0, -9_000.0, 6_000.0),
            yaw: 45.0_f32.to_radians(),
            pitch: -28.0_f32.to_radians(),
            speed: 4_000.0,
            fov_y: 58.0_f32.to_radians(),
        }
    }
}

impl Camera3d {
    pub(crate) fn look_at(position: Vec3, target: Vec3, speed: f32) -> Self {
        let to_target = target - position;
        let direction = to_target.normalized();
        let yaw = direction.y.atan2(direction.x);
        let horizontal = (direction.x * direction.x + direction.y * direction.y).sqrt();
        let pitch = direction.z.atan2(horizontal).clamp(-1.45, 1.45);
        Self {
            position,
            yaw,
            pitch,
            speed: speed.max(100.0),
            fov_y: 58.0_f32.to_radians(),
        }
    }

    pub(crate) fn forward(self) -> Vec3 {
        let cp = self.pitch.cos();
        Vec3::new(self.yaw.cos() * cp, self.yaw.sin() * cp, self.pitch.sin()).normalized()
    }

    pub(crate) fn basis(self) -> CameraBasis {
        let forward = self.forward();
        let right = Vec3::new(-self.yaw.sin(), self.yaw.cos(), 0.0).normalized();
        let up = forward.cross(right).normalized();
        CameraBasis { forward, right, up }
    }

    pub(crate) fn look_delta(&mut self, delta: UiPoint, sensitivity: f32) {
        self.yaw += delta.x * sensitivity;
        self.pitch = (self.pitch - delta.y * sensitivity).clamp(-1.45, 1.45);
    }

    pub(crate) fn scroll_forward(&mut self, scroll: f32) {
        self.position += self.forward() * scroll * self.speed * 0.0015;
    }
}

pub(crate) fn view_projection_3d(camera: Camera3d, aspect: f32, far: f32) -> [f32; 16] {
    let basis = camera.basis();
    let y_scale = 1.0 / (camera.fov_y * 0.5).tan();
    let x_scale = y_scale / aspect.max(0.001);
    let near = CAMERA_NEAR_PLANE.max(0.001);
    let far = far.max(near + 1.0);
    let z_scale = far / (far - near);
    let z_bias = -near * far / (far - near);
    let position = camera.position;
    row_major_4x4_to_column_major([
        [
            basis.right.x * x_scale,
            basis.right.y * x_scale,
            basis.right.z * x_scale,
            -position.dot(basis.right) * x_scale,
        ],
        [
            basis.up.x * y_scale,
            basis.up.y * y_scale,
            basis.up.z * y_scale,
            -position.dot(basis.up) * y_scale,
        ],
        [
            basis.forward.x * z_scale,
            basis.forward.y * z_scale,
            basis.forward.z * z_scale,
            -position.dot(basis.forward) * z_scale + z_bias,
        ],
        [
            basis.forward.x,
            basis.forward.y,
            basis.forward.z,
            -position.dot(basis.forward),
        ],
    ])
}

fn row_major_4x4_to_column_major(matrix: [[f32; 4]; 4]) -> [f32; 16] {
    let mut out = [0.0; 16];
    for row in 0..4 {
        for column in 0..4 {
            out[column * 4 + row] = matrix[row][column];
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Vec3 {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
}

impl Vec3 {
    pub(crate) const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub(crate) const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub(crate) fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub(crate) fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub(crate) fn normalized(self) -> Self {
        let length = self.length();
        if length <= f32::EPSILON {
            return Self::ZERO;
        }
        self / length
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}
