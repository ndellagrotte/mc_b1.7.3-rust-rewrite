//! `SignModel` (SignModel.java): the board (24×12×2 at texture 0, 0) and
//! the stick (2×14×2 at texture 0, 14) of `/item/sign.png`. Drawn through
//! [`super::gl::EntityGl::render_part`].

use crate::model::ModelPart;

/// `SignModel`.
#[derive(Clone, Debug, PartialEq)]
pub struct SignModel {
    /// `signBoard`.
    pub sign_board: ModelPart,
    /// `signStick`.
    pub sign_stick: ModelPart,
}

impl Default for SignModel {
    // src: SignModel.java:7-11
    fn default() -> Self {
        let mut sign_board = ModelPart::new("signBoard", 0, 0);
        sign_board.add_box(-12.0, -14.0, -1.0, 24, 12, 2);
        let mut sign_stick = ModelPart::new("signStick", 0, 14);
        sign_stick.add_box(-1.0, -2.0, -1.0, 2, 14, 2);
        SignModel {
            sign_board,
            sign_stick,
        }
    }
}

impl SignModel {
    /// `func_887_a()`: both parts at 1/16.
    // src: SignModel.java:13-16
    pub fn func_887_a(&self, gl: &mut super::gl::EntityGl) {
        gl.render_part(&self.sign_board, 0.0625);
        gl.render_part(&self.sign_stick, 0.0625);
    }
}
