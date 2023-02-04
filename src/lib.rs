#![no_std]
use asr::{
    timer,
    //timer::TimerState,
    watcher::Watcher,
    Address,
    Process,
};
use spinning_top::{const_spinlock, Spinlock};

#[cfg(all(not(test), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

static AUTOSPLITTER: Spinlock<State> = const_spinlock(State {
    game: None,
    watchers: Watchers {
        is_loading: Watcher::new(),
        fade: Watcher::new(),
    },
    // settings: None,
});

struct State {
    game: Option<ProcessInfo>,
    watchers: Watchers,
    // settings: Settings,
}

struct Watchers {
    is_loading: Watcher<u8>,
    fade: Watcher<f32>,
    // other
}

struct ProcessInfo {
    game: Process,
    main_module_base: Address,
    //main_module_size: u64,
}

impl State {
    fn attach_process() -> Option<ProcessInfo> {
        //let game_name_2 = "whatever.exe";
        //let game_process = Process::attach(game_name).or_else(|| Process::attach(game_name_2))?;

        //let game = Process::attach(game_name)?;

        let game_names = ["Need For Speed The Run.exe"];

        let mut proc: Option<Process> = None;
        let mut curgamename: &str = "";
        
        for name in game_names {
            let iproc = Process::attach(name);
            if iproc.is_some() {
                proc = iproc;
                curgamename = name;
                break;
            }
        }
        
        let Some(game) = proc else { return None };

        let main_module_base = game.get_module_address(curgamename).ok()?;

        Some(ProcessInfo {
            game,
            main_module_base,
            //main_module_size: game_main_module_size,
        })
    }

    fn update(&mut self) {
        // Attaching to target process
        if self.game.is_none() {
            self.game = State::attach_process()
        }

        let Some(game) = &self.game else {
            return;
        };
        let proc = &game.game;
        if !proc.is_open() {
            self.game = None;
            return;
        };

        // Update
        let Address(base_address) = game.main_module_base;
        let Some(is_loading) = self.watchers.is_loading.update(proc.read(Address(base_address + 0x24682E4)).ok()) else { return };
        let Some(fade) = self.watchers.fade.update(proc.read(Address(base_address + 0x248BBC4)).ok()) else { return };

        if is_loading.current == 0 {
            timer::resume_game_time()
        } else if fade.current == 1.0 {
            timer::pause_game_time()
        } else {
            timer::resume_game_time()
        }
    }
}

#[no_mangle]
pub extern "C" fn update() {
    AUTOSPLITTER.lock().update();
}