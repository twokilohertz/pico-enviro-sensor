use embassy_futures::select::{select3, Either3};
use embassy_rp::gpio::Input;
use embassy_time::{with_timeout, Duration, Instant};
use rtt_target::debug_rprintln;

const DEBOUNCE_TIME_MILLIS: u64 = 20;

#[embassy_executor::task]
pub async fn input_handling_task(
    mut button_enter: Input<'static>,
    mut button_left: Input<'static>,
    mut button_right: Input<'static>,
) {
    debug_rprintln!("Input handling task started");

    loop {
        let enter_pressed = button_enter.wait_for_falling_edge();
        let left_pressed = button_left.wait_for_falling_edge();
        let right_pressed = button_right.wait_for_falling_edge();

        match select3(enter_pressed, left_pressed, right_pressed).await {
            Either3::First(_) => {
                let pressed_at = Instant::now();
                if with_timeout(Duration::from_secs(2), button_enter.wait_for_rising_edge())
                    .await
                    .is_ok()
                {
                    if pressed_at.elapsed().as_millis() < DEBOUNCE_TIME_MILLIS {
                        continue;
                    }
                    debug_rprintln!("Enter button pressed!");
                }
            }
            Either3::Second(_) => {
                let pressed_at = Instant::now();
                if with_timeout(Duration::from_secs(2), button_left.wait_for_rising_edge())
                    .await
                    .is_ok()
                {
                    if pressed_at.elapsed().as_millis() < DEBOUNCE_TIME_MILLIS {
                        continue;
                    }
                    debug_rprintln!("Left button pressed!");
                }
            }
            Either3::Third(_) => {
                let pressed_at = Instant::now();
                if with_timeout(Duration::from_secs(2), button_right.wait_for_rising_edge())
                    .await
                    .is_ok()
                {
                    if pressed_at.elapsed().as_millis() < DEBOUNCE_TIME_MILLIS {
                        continue;
                    }
                    debug_rprintln!("Right button pressed!");
                }
            }
        };
    }
}
