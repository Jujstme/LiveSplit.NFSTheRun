#![no_std]
use asr::{signature::Signature, timer, timer::TimerState, watcher::Watcher, Address, Process};

#[cfg(all(not(test), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

static AUTOSPLITTER: spinning_top::Spinlock<State> = spinning_top::const_spinlock(State {
    game: None,
    sigscans: None,
    watchers: Watchers {
        state: Watcher::new(),
        fade: Watcher::new(),
    },
    // settings: None,
});

struct State {
    game: Option<ProcessInfo>,
    sigscans: Option<SigScan>,
    watchers: Watchers,
    // settings: Settings,
}

struct Watchers {
    state: Watcher<u8>,
    fade: Watcher<f32>,
}

struct ProcessInfo {
    game: Process,
    main_module_base: Address,
    //main_module_size: u64,
}

impl State {
    fn attach_process() -> Option<ProcessInfo> {
        const PROCESS_NAMES: [&str; 1] = ["Need For Speed The Run.exe"];
        let mut proc: Option<Process> = None;

        for name in PROCESS_NAMES {
            proc = Process::attach(name);
            if proc.is_some() {
                break;
            }
        }

        let game = proc?;

        // Sets the tick rate to 60hz because I think 120 (the default value) is overkill for this autosplitter
        asr::set_tick_rate(60.0);

        Some(ProcessInfo {
            game,
            main_module_base: Address(0x400000), // Querying it fails for some reason, but as this game doesn't use ASLR, the base address is always the same
            //main_module_size: game.get_module_size(curgamename).ok()?, // currently broken in livesplit classic
        })
    }

    fn update(&mut self) {
        // Checks is LiveSplit is currently attached to a target process and runs attach_process() otherwise
        if self.game.is_none() {
            self.game = State::attach_process()
        }
        let Some(game) = &self.game else { return };
        let proc = &game.game;

        if !proc.is_open() {
            self.game = None;
            if timer::state() == TimerState::Running { timer::pause_game_time() } // If the game crashes, game time should be paused
            return;
        };

        // Update
        // Look for valid sigscans and performs sigscan if necessary
        let Some(addresses) = &self.sigscans else { self.sigscans = SigScan::new(proc, game.main_module_base); return; };

        // Update the watchers variables
        let Some(state) = self.watchers.state.update(proc.read_pointer_path32(addresses.state.0 as u32, &[0x0, 0x44, 0x18]).ok()) else { return };
        let Some(fade) = self.watchers.fade.update(proc.read(addresses.fade).ok()) else { return };

        // Splitting logic
        match timer::state() {
            TimerState::Running => {
                if state.current == 1 && fade.current == 1.0 {
                    timer::pause_game_time()
                } else {
                    timer::resume_game_time()
                }
            }
            _ => {}
        }
    }
}

#[no_mangle]
pub extern "C" fn update() {
    AUTOSPLITTER.lock().update();
}

struct SigScan {
    state: Address,
    fade: Address,
}

impl SigScan {
    fn new(process: &Process, addr: Address) -> Option<Self> {
        let size = 0x3200000; // Hack, until we can actually query ModuleMemorySize
 
        const SIG: Signature<8> = Signature::new("A1 ???????? D9 45 08");
        let mut scan = SIG.scan_process_range(process, addr, size)?;
        let state = process.read::<u32>(Address(scan.0 + 1)).ok()? as u64;

        const SIG_FADE: Signature<14> = Signature::new("E8 ???????? E8 ???????? 38 5C 24 14");
        scan = SIG_FADE.scan_process_range(process, addr, size)?;
        let mut fade = process.read::<u32>(Address(scan.0 + 1)).ok()? as u64;
        fade = process.read::<u32>(Address(scan.0 + 0x5 + fade + 0x14)).ok()? as u64;


        Some(Self {
            state: Address(state),
            fade: Address(fade + 0x4),
        })
    }
}