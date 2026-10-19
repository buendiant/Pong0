//! Simulación autoritativa pura. No conoce sockets, GUI ni relojes del sistema.
use crate::protocol::{Action, Side, StateStatus, StateSync};
use std::time::Duration;

pub const WINNING_SCORE: u8 = 7;
const PADDLE_SPEED: f32 = 360.0;
const PADDLE_HALF_HEIGHT: f32 = 45.0;
const BALL_RADIUS: f32 = 6.0;
const POINT_PAUSE: Duration = Duration::from_millis(800);

pub struct Game {
    state: StateSync,
    velocity_x: f32,
    velocity_y: f32,
    actions: [Action; 2],
    pause_remaining: Duration,
    winner: Option<Side>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self {
            state: StateSync {
                ball_x: 400.0,
                ball_y: 300.0,
                paddle_left_y: 300.0,
                paddle_right_y: 300.0,
                score_left: 0,
                score_right: 0,
                status: StateStatus::InGame,
            },
            velocity_x: 280.0,
            velocity_y: 180.0,
            actions: [Action::Stop; 2],
            pause_remaining: Duration::ZERO,
            winner: None,
        }
    }

    pub fn set_input(&mut self, side: Side, action: Action) {
        self.actions[side as usize - 1] = action;
    }

    pub fn snapshot(&self) -> StateSync {
        self.state.clone()
    }
    pub fn winner(&self) -> Option<Side> {
        self.winner
    }

    /// Subpasos de hasta 8 ms evitan atravesar palas incluso con ticks lentos.
    /// El llamador fija el tiempo; nunca se acepta un delta enviado por clientes.
    pub fn advance(&mut self, elapsed: Duration) {
        if self.winner.is_some() {
            return;
        }
        let mut remaining = elapsed.min(Duration::from_millis(100));
        while !remaining.is_zero() && self.winner.is_none() {
            let step = remaining.min(Duration::from_millis(8));
            self.step(step);
            remaining -= step;
        }
    }

    fn step(&mut self, elapsed: Duration) {
        let dt = elapsed.as_secs_f32();
        for (paddle, action) in [
            (&mut self.state.paddle_left_y, self.actions[0]),
            (&mut self.state.paddle_right_y, self.actions[1]),
        ] {
            let direction = match action {
                Action::Stop => 0.0,
                Action::MoveUp => -1.0,
                Action::MoveDown => 1.0,
            };
            *paddle = (*paddle + direction * PADDLE_SPEED * dt).clamp(45.0, 555.0);
        }
        if !self.pause_remaining.is_zero() {
            self.pause_remaining = self.pause_remaining.saturating_sub(elapsed);
            if self.pause_remaining.is_zero() {
                self.state.status = StateStatus::InGame;
            }
            return;
        }

        let old_x = self.state.ball_x;
        self.state.ball_x += self.velocity_x * dt;
        self.state.ball_y += self.velocity_y * dt;

        if self.state.ball_y < BALL_RADIUS {
            self.state.ball_y = 2.0 * BALL_RADIUS - self.state.ball_y;
            self.velocity_y = self.velocity_y.abs();
        } else if self.state.ball_y > 600.0 - BALL_RADIUS {
            self.state.ball_y = 2.0 * (600.0 - BALL_RADIUS) - self.state.ball_y;
            self.velocity_y = -self.velocity_y.abs();
        }

        let hit_left = self.velocity_x < 0.0
            && old_x >= 42.0
            && self.state.ball_x <= 42.0
            && (self.state.ball_y - self.state.paddle_left_y).abs()
                <= PADDLE_HALF_HEIGHT + BALL_RADIUS;
        let hit_right = self.velocity_x > 0.0
            && old_x <= 758.0
            && self.state.ball_x >= 758.0
            && (self.state.ball_y - self.state.paddle_right_y).abs()
                <= PADDLE_HALF_HEIGHT + BALL_RADIUS;
        if hit_left || hit_right {
            let paddle_y = if hit_left {
                self.state.paddle_left_y
            } else {
                self.state.paddle_right_y
            };
            let offset = ((self.state.ball_y - paddle_y) / PADDLE_HALF_HEIGHT).clamp(-1.0, 1.0);
            self.velocity_y = offset * 320.0;
            let speed = (self.velocity_x.abs() * 1.05).min(600.0);
            self.velocity_x = if hit_left { speed } else { -speed };
            self.state.ball_x = if hit_left { 42.0 } else { 758.0 };
        }

        if self.state.ball_x < 0.0 {
            self.score(Side::Right);
        } else if self.state.ball_x > 800.0 {
            self.score(Side::Left);
        }
    }

    fn score(&mut self, scorer: Side) {
        match scorer {
            Side::Left => self.state.score_left += 1,
            Side::Right => self.state.score_right += 1,
        }
        self.state.ball_x = 400.0;
        self.state.ball_y = 300.0;
        if self.state.score_left >= WINNING_SCORE || self.state.score_right >= WINNING_SCORE {
            self.winner = Some(scorer);
            self.state.status = StateStatus::Finished;
        } else {
            self.state.status = match scorer {
                Side::Left => StateStatus::LeftPoint,
                Side::Right => StateStatus::RightPoint,
            };
            self.pause_remaining = POINT_PAUSE;
            self.velocity_x = if scorer == Side::Left { 280.0 } else { -280.0 };
            self.velocity_y = 150.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_is_bounded_and_stop_is_respected() {
        let mut game = Game::new();
        game.set_input(Side::Left, Action::MoveUp);
        game.set_input(Side::Right, Action::MoveDown);
        for _ in 0..20 {
            game.advance(Duration::from_millis(100));
        }
        assert_eq!(game.snapshot().paddle_left_y, 45.0);
        assert_eq!(game.snapshot().paddle_right_y, 555.0);
        game.set_input(Side::Left, Action::Stop);
        game.advance(Duration::from_millis(50));
        assert_eq!(game.snapshot().paddle_left_y, 45.0);
    }

    #[test]
    fn wall_and_paddle_bounces_are_authoritative() {
        let mut game = Game::new();
        game.state.ball_y = 7.0;
        game.velocity_y = -180.0;
        game.advance(Duration::from_millis(16));
        assert!(game.velocity_y > 0.0);
        assert!(game.snapshot().ball_y >= BALL_RADIUS);
        for side in [Side::Left, Side::Right] {
            game.state.ball_x = if side == Side::Left { 43.0 } else { 757.0 };
            game.state.ball_y = 300.0;
            game.velocity_y = 0.0;
            game.velocity_x = if side == Side::Left { -600.0 } else { 600.0 };
            game.advance(Duration::from_millis(16));
            assert_eq!(game.velocity_x > 0.0, side == Side::Left);
            assert_eq!(game.snapshot().score_left + game.snapshot().score_right, 0);
        }
    }

    #[test]
    fn goals_pause_reset_and_finish_exactly_at_seven() {
        let mut game = Game::new();
        for score in 1..=WINNING_SCORE {
            game.state.ball_x = 799.0;
            game.state.ball_y = 100.0;
            game.velocity_x = 280.0;
            game.velocity_y = 0.0;
            game.advance(Duration::from_millis(16));
            assert_eq!(game.snapshot().score_left, score);
            assert_eq!(game.snapshot().ball_x, 400.0);
            if score < WINNING_SCORE {
                assert_eq!(game.snapshot().status, StateStatus::LeftPoint);
                for _ in 0..8 {
                    game.advance(Duration::from_millis(100));
                }
                assert_eq!(game.snapshot().status, StateStatus::InGame);
            }
        }
        assert_eq!(game.winner(), Some(Side::Left));
        let finished = game.snapshot();
        game.advance(Duration::from_secs(1));
        assert_eq!(game.snapshot(), finished);
    }

    #[test]
    fn a_goal_on_the_left_awards_the_right_player() {
        let mut game = Game::new();
        game.state.ball_x = 1.0;
        game.state.ball_y = 100.0;
        game.velocity_x = -280.0;
        game.advance(Duration::from_millis(16));
        assert_eq!(game.snapshot().score_right, 1);
        assert_eq!(game.snapshot().status, StateStatus::RightPoint);
    }

    #[test]
    fn matches_do_not_share_state() {
        let mut first = Game::new();
        let second = Game::new();
        first.set_input(Side::Left, Action::MoveUp);
        first.advance(Duration::from_millis(100));
        assert!(first.snapshot().paddle_left_y < 300.0);
        assert_eq!(second.snapshot().paddle_left_y, 300.0);
        assert_eq!(second.snapshot().ball_x, 400.0);
    }
}
