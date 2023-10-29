#![no_std]
#![feature(type_alias_impl_trait, const_async_blocks)]
#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::style,
    clippy::undocumented_unsafe_blocks,
    rust_2018_idioms
)]

use asr::{
    file_format::pe,
    future::{next_tick, retry},
    signature::Signature,
    time::Duration,
    timer::{self, TimerState},
    watcher::Watcher,
    Address, Address32, Process,
};

asr::panic_handler!();
asr::async_main!(nightly);

const PROCESS_NAMES: &[&str] = &["Need For Speed The Run.exe"];

async fn main() {
    asr::set_tick_rate(60.0);

    loop {
        // Hook to the target process
        let process = retry(|| PROCESS_NAMES.iter().find_map(|&name| Process::attach(name))).await;

        process
            .until_closes(async {
                // Once the target has been found and attached to, set up some default watchers
                let mut watchers = Watchers::default();

                // Perform memory scanning to look for the addresses we need
                let addresses = Addresses::init(&process).await;

                loop {
                    // Splitting logic. Adapted from OG LiveSplit:
                    // Order of execution
                    // 1. update() will always be run first. There are no conditions on the execution of this action.
                    // 2. If the timer is currently either running or paused, then the isLoading, gameTime, and reset actions will be run.
                    // 3. If reset does not return true, then the split action will be run.
                    // 4. If the timer is currently not running (and not paused), then the start action will be run.
                    update_loop(&process, &addresses, &mut watchers);

                    let timer_state = timer::state();
                    if timer_state == TimerState::Running || timer_state == TimerState::Paused {
                        if let Some(is_loading) = is_loading(&watchers) {
                            if is_loading {
                                timer::pause_game_time()
                            } else {
                                timer::resume_game_time()
                            }
                        }

                        if let Some(game_time) = game_time(&watchers, &addresses) {
                            timer::set_game_time(game_time)
                        }

                        if reset(&watchers) {
                            timer::reset()
                        } else if split(&watchers) {
                            timer::split()
                        }
                    }

                    if timer::state() == TimerState::NotRunning && start(&watchers) {
                        timer::start();
                        timer::pause_game_time();

                        if let Some(is_loading) = is_loading(&watchers) {
                            if is_loading {
                                timer::pause_game_time()
                            } else {
                                timer::resume_game_time()
                            }
                        }
                    }

                    next_tick().await;
                }
            })
            .await;
    }
}

#[derive(Default)]
struct Watchers {
    state: Watcher<u8>,
    fade: Watcher<f32>,
}

struct Addresses {
    state: Address,
    fade: Address,
}

impl Addresses {
    async fn init(process: &Process) -> Self {
        let main_module = {
            let main_module_base = Address::new(0x400000); // Querying it fails for some reason, but as this game doesn't use ASLR, the base address is always the same
            let main_module_size = retry(|| pe::read_size_of_image(process, main_module_base)).await; // 0x3200000
            (main_module_base, main_module_size as u64)
        };

        const SIG: Signature<8> = Signature::new("A1 ???????? D9 45 08");
        let mut scan = retry(|| SIG.scan_process_range(process, main_module)).await;
        let state = retry(|| process.read::<Address32>(scan + 1)).await.into();

        const SIG_FADE: Signature<14> = Signature::new("E8 ???????? E8 ???????? 38 5C 24 14");
        scan = retry(|| SIG_FADE.scan_process_range(process, main_module)).await;
        let mut fade = retry(|| process.read::<Address32>(scan + 1)).await;
        fade = retry(|| process.read::<Address32>(scan + 0x5 + fade.value() + 0x14)).await;

        Self {
            state,
            fade: fade.add(0x4).into(),
        }
    }
}

fn update_loop(process: &Process, addresses: &Addresses, watchers: &mut Watchers) {
    watchers.state.update_infallible(
        process
            .read_pointer_path32(addresses.state, &[0x0, 0x44, 0x18])
            .unwrap_or_default(),
    );
    watchers
        .fade
        .update_infallible(process.read(addresses.fade).unwrap_or_default());
}

fn start(_watchers: &Watchers) -> bool {
    false
}

fn split(_watchers: &Watchers) -> bool {
    false
}

fn reset(_watchers: &Watchers) -> bool {
    false
}

fn is_loading(watchers: &Watchers) -> Option<bool> {
    Some(watchers.state.pair?.current == 1 && watchers.fade.pair?.current == 1.0)
}

fn game_time(_watchers: &Watchers, _addresses: &Addresses) -> Option<Duration> {
    None
}
