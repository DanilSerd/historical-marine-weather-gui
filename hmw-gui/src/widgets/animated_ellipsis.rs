use std::time::Duration;

use iced::Subscription;

const FRAME_COUNT: usize = 4;
const DEFAULT_INTERVAL_MS: u64 = 300;

/// Cycles a trailing ellipsis so loading labels can animate over time.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AnimatedEllipsis {
    frame: usize,
    interval: Duration,
}

impl Default for AnimatedEllipsis {
    fn default() -> Self {
        Self {
            frame: FRAME_COUNT - 1,
            interval: Duration::from_millis(DEFAULT_INTERVAL_MS),
        }
    }
}

impl AnimatedEllipsis {
    /// Advances the animation by one frame.
    pub(crate) fn tick(&mut self) {
        self.frame = (self.frame + 1) % FRAME_COUNT;
    }

    /// Resets the animation to its starting frame.
    pub(crate) fn reset(&mut self) {
        self.frame = FRAME_COUNT - 1;
    }

    /// Builds the current label text with the animated trailing dots.
    pub(crate) fn text(&self, label: &str) -> String {
        format!("{label}{}", ".".repeat(self.frame))
    }

    /// Emits timer ticks while the animation is enabled.
    pub(crate) fn subscription(&self, enabled: bool) -> Subscription<()> {
        match enabled {
            true => iced::time::every(self.interval).map(|_| ()),
            false => Subscription::none(),
        }
    }
}
