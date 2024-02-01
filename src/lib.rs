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
    deep_pointer::DeepPointer,
    future::{next_tick, retry},
    settings::{gui::Title, Gui},
    string::ArrayCString,
    time::Duration,
    timer::{self, TimerState},
    watcher::Watcher,
    Address, PointerSize, Process,
};

asr::panic_handler!();
asr::async_main!(nightly);

const PROCESS_NAMES: &[&str] = &["Need For Speed The Run.exe"];

async fn main() {
    let mut settings = Settings::register();
    let mut watchers = Watchers::default();

    loop {
        // Hook to the target process
        let (process_name, process) = retry(|| {
            PROCESS_NAMES
                .iter()
                .find_map(|&name| Some((name, Process::attach(name)?)))
        })
        .await;

        process
            .until_closes(async {
                // Once the target has been found and attached to, set up some default watchers
                watchers.split_buffer = 0;

                // Perform memory scanning to look for the addresses we need
                let memory = Memory::init(&process, process_name).await;

                loop {
                    // Splitting logic. Adapted from OG LiveSplit:
                    // Order of execution
                    // 1. update() will always be run first. There are no conditions on the execution of this action.
                    // 2. If the timer is currently either running or paused, then the isLoading, gameTime, and reset actions will be run.
                    // 3. If reset does not return true, then the split action will be run.
                    // 4. If the timer is currently not running (and not paused), then the start action will be run.
                    settings.update();
                    update_loop(&process, &memory, &mut watchers);

                    if [TimerState::Running, TimerState::Paused].contains(&timer::state()) {
                        match is_loading(&watchers, &settings) {
                            Some(true) => timer::pause_game_time(),
                            Some(false) => timer::resume_game_time(),
                            _ => (),
                        }

                        match game_time(&watchers, &settings, &memory) {
                            Some(x) => timer::set_game_time(x),
                            _ => (),
                        }

                        match reset(&watchers, &settings) {
                            true => timer::reset(),
                            _ => match split(&mut watchers, &settings) {
                                true => timer::split(),
                                _ => (),
                            },
                        }
                    }

                    if timer::state().eq(&TimerState::NotRunning) && start(&watchers, &settings) {
                        timer::start();
                        timer::pause_game_time();

                        match is_loading(&watchers, &settings) {
                            Some(true) => timer::pause_game_time(),
                            Some(false) => timer::resume_game_time(),
                            _ => (),
                        }
                    }

                    next_tick().await;
                }
            })
            .await;

        timer::pause_game_time();
    }
}

#[derive(Gui)]
struct Settings {
    /// General settings
    _general: Title,
    #[default = true]
    /// Enable auto start
    start: bool,
    #[default = true]
    /// Enable auto splitting
    split: bool,
    /// Split Settings
    _split: Title,
    #[default = true]
    /// 1.1 - MOB escape
    story_1_1: bool,
    #[default = true]
    /// 1.2 - Nob Hill
    story_1_2: bool,
    #[default = true]
    /// 1.3 - Altamont Park
    story_1_3: bool,
    #[default = true]
    /// 1.4 - Interstate 580
    story_1_4: bool,
    #[default = true]
    /// 2.1 - 140 Hwy
    story_2_1: bool,
    #[default = true]
    /// 2.2 - El Portal Rd
    story_2_2: bool,
    #[default = true]
    /// 2.3 - El Capitan
    story_2_3: bool,
    #[default = true]
    /// 2.4 - Tioga Pass Rd
    story_2_4: bool,
    #[default = true]
    /// 2.5 - Ellery Lake
    story_2_5: bool,
    #[default = true]
    /// 3.1 - Panamint Valley
    story_3_1: bool,
    #[default = true]
    /// 3.2 - Junction Rd
    story_3_2: bool,
    #[default = true]
    /// 3.3 - Old Spanish Trail
    story_3_3: bool,
    #[default = true]
    /// 3.4 - Nikki and Mila
    story_3_4: bool,
    #[default = true]
    /// 3.5 - Las Vegas Blvd
    story_3_5: bool,
    #[default = true]
    /// 3.6 - Northshore Rd (2 splits)
    story_3_6: bool,
    #[default = true]
    /// 4.1 - Northshore Rd
    story_4_1: bool,
    #[default = true]
    /// 4.2 - Hwy 169
    story_4_2: bool,
    #[default = true]
    /// 4.3 - Rockville
    story_4_3: bool,
    #[default = true]
    /// 4.4 - Red Mountain Pass
    story_4_4: bool,
    #[default = true]
    /// 4.5 - Loghill
    story_4_5: bool,
    #[default = true]
    /// 5.1 - Interstate 70
    story_5_1: bool,
    #[default = true]
    /// 5.2 - Glenwood Springs
    story_5_2: bool,
    #[default = true]
    /// 5.3 - Route 82
    story_5_3: bool,
    #[default = true]
    /// 5.4 - Aspen
    story_5_4: bool,
    #[default = true]
    /// 5.5 - Independence Pass
    story_5_5: bool,
    #[default = true]
    /// 6.1 - Independence Pass
    story_6_1: bool,
    #[default = true]
    /// 6.2 - Highway 20
    story_6_2: bool,
    #[default = true]
    /// 6.3 - South Dakota 44
    story_6_3: bool,
    #[default = true]
    /// 6.4 - South Dakota 240
    story_6_4: bool,
    #[default = true]
    /// 6.5 - Country Rd 25
    story_6_5: bool,
    #[default = true]
    /// 7.1 - Country Hwy
    story_7_1: bool,
    #[default = true]
    /// 7.2 - Riverside Dr
    story_7_2: bool,
    #[default = true]
    /// 7.3 - Kennedy Expressway
    story_7_3: bool,
    #[default = true]
    /// 7.4 - Lower Wacker (4 splits)
    story_7_4: bool,
    #[default = true]
    /// 8.1 - Downtown
    story_8_1: bool,
    #[default = true]
    /// 8.2 - Lakeshore Drive
    story_8_2: bool,
    #[default = true]
    /// 8.3 - Interstate 75
    story_8_3: bool,
    #[default = true]
    /// 8.4 - Northwest Freeway
    story_8_4: bool,
    #[default = true]
    /// 8.5 - Industrial District
    story_8_5: bool,
    #[default = true]
    /// 9.1 - Expressway
    story_9_1: bool,
    #[default = true]
    /// 9.2 - Interstate 68
    story_9_2: bool,
    #[default = true]
    /// 9.3 - Deer Park
    story_9_3: bool,
    #[default = true]
    /// 9.4 - Samwill Drive
    story_9_4: bool,
    #[default = true]
    /// 10.1 - Pine Grove Rd
    story_10_1: bool,
    #[default = true]
    /// 10.2 - Interstate 78 Express
    story_10_2: bool,
    #[default = true]
    /// 10.3 - Union
    story_10_3: bool,
    #[default = true]
    /// 10.4 - Cesar
    story_10_4: bool,
    #[default = true]
    /// 10.5 - Calvin Garret
    story_10_5: bool,
    #[default = true]
    /// 10.6 - Marcus
    story_10_6: bool,
}

#[derive(Default)]
struct Watchers {
    is_loading: Watcher<bool>,
    story_mode_flag: Watcher<bool>,
    stage_id: Watcher<StageID>,
    race_won: Watcher<bool>,
    split_buffer: u8,
}

struct Memory {
    is_loading: DeepPointer<3>,
    story_mode_flag: DeepPointer<2>,
    stage_id: DeepPointer<4>,
    race_won: DeepPointer<9>,
}

impl Memory {
    async fn init(_process: &Process, _process_name: &str) -> Self {
        #[allow(non_upper_case_globals)]
        const main_module_base: Address = Address::new(0x400000);

        let is_loading = DeepPointer::new(
            main_module_base,
            PointerSize::Bit32,
            &[0x267FBB8, 0x44, 0x8C],
        );

        let story_mode_flag =
            DeepPointer::new(main_module_base, PointerSize::Bit32, &[0x24824E8, 0x14]);

        let stage_id = DeepPointer::new(
            main_module_base,
            PointerSize::Bit32,
            &[0x24824E8, 0xC, 0xC, 0x0],
        );

        let race_won = DeepPointer::new(
            main_module_base,
            PointerSize::Bit32,
            &[0x2353F18, 0x10C, 0x4, 0x14, 0x128, 0x28, 0xA8, 0x0, 0x60],
        );

        Self {
            is_loading,
            story_mode_flag,
            stage_id,
            race_won,
        }
    }
}

fn update_loop(process: &Process, memory: &Memory, watchers: &mut Watchers) {
    watchers.is_loading.update_infallible(
        memory
            .is_loading
            .deref::<u8>(process)
            .is_ok_and(|val| val == 1),
    );
    watchers.story_mode_flag.update_infallible(
        memory
            .story_mode_flag
            .deref::<u8>(process)
            .is_ok_and(|val| val == 1),
    );

    let stage_id = watchers.stage_id.update_infallible(
        match memory.stage_id.deref::<ArrayCString<255>>(process) {
            Ok(val) => {
                let chapter = val.as_bytes();
                let c = chapter
                    .iter()
                    .rev()
                    .position(|&val| val.eq(&b'/'))
                    .unwrap_or(1);
                let chapter = &chapter[chapter.len().saturating_sub(c)..];

                match chapter {
                    b"1_1" => StageID::Chapter1_1,
                    b"1_5" => StageID::Chapter1_2,
                    b"2_2" => StageID::Chapter1_3,
                    b"2_1" => StageID::Chapter1_4,
                    b"2_3A" => StageID::Chapter2_1,
                    b"2_3B" => StageID::Chapter2_2,
                    b"2_4" => StageID::Chapter2_3,
                    b"2_5" => StageID::Chapter2_4,
                    b"2_6" => StageID::Chapter2_5,
                    b"3_1" => StageID::Chapter3_1,
                    b"3_2" => StageID::Chapter3_2,
                    b"3_3" => StageID::Chapter3_3,
                    b"3_41" => StageID::Chapter3_4,
                    b"3_5" => StageID::Chapter3_5,
                    b"3_51" => StageID::Chapter3_6,
                    b"4_3A" => StageID::Chapter4_1,
                    b"4_1" => StageID::Chapter4_2,
                    b"5_1" => StageID::Chapter4_3,
                    b"5_2" => StageID::Chapter4_4,
                    b"5_3" => StageID::Chapter4_5,
                    b"6_1" => StageID::Chapter5_1,
                    b"6_2" => StageID::Chapter5_2,
                    b"6_21" => StageID::Chapter5_3,
                    b"6_22" => StageID::Chapter5_4,
                    b"6_3" => StageID::Chapter5_5,
                    b"9_0" => StageID::Chapter6_1,
                    b"9_1" => StageID::Chapter6_2,
                    b"9_21" => StageID::Chapter6_3,
                    b"9_22" => StageID::Chapter6_4,
                    b"9_3" => StageID::Chapter6_5,
                    b"10_1" => StageID::Chapter7_1,
                    b"10_2" => StageID::Chapter7_2,
                    b"10_3" => StageID::Chapter7_3,
                    b"10_31" => StageID::Chapter7_4,
                    b"11_0" => StageID::Chapter8_1,
                    b"11_11" => StageID::Chapter8_2,
                    b"11_2" => StageID::Chapter8_3,
                    b"11_3" => StageID::Chapter8_4,
                    b"11_4" => StageID::Chapter8_5,
                    b"12_1" => StageID::Chapter9_1,
                    b"12_2" => StageID::Chapter9_2,
                    b"12_31" => StageID::Chapter9_3,
                    b"12_32" => StageID::Chapter9_4,
                    b"13_1" => StageID::Chapter10_1,
                    b"13_2" => StageID::Chapter10_2,
                    b"13_3" => StageID::Chapter10_3,
                    b"13_31" => StageID::Chapter10_4,
                    b"13_4" => StageID::Chapter10_5,
                    b"13_41" => StageID::Chapter10_6,
                    _ => match &watchers.stage_id.pair {
                        Some(x) => x.current,
                        None => StageID::Chapter1_1,
                    },
                }
            }
            _ => match &watchers.stage_id.pair {
                Some(x) => x.current,
                None => StageID::Chapter1_1,
            },
        },
    );

    watchers
        .race_won
        .update_infallible(memory.race_won.deref::<u8>(process).unwrap_or_default() == 1);

    if stage_id.changed() {
        watchers.split_buffer = 0;
    }
}

fn start(watchers: &Watchers, settings: &Settings) -> bool {
    settings.start
        && watchers
            .story_mode_flag
            .pair
            .is_some_and(|val| val.changed_to(&true))
        && watchers.is_loading.pair.is_some_and(|val| !val.current)
}

fn split(watchers: &mut Watchers, settings: &Settings) -> bool {
    settings.split
        && watchers
            .race_won
            .pair
            .is_some_and(|val| val.changed_to(&true))
        && watchers.stage_id.pair.is_some_and(|val| match val.current {
            StageID::Chapter1_1 => settings.story_1_1,
            StageID::Chapter1_2 => settings.story_1_2,
            StageID::Chapter1_3 => settings.story_1_3,
            StageID::Chapter1_4 => settings.story_1_4,
            StageID::Chapter2_1 => settings.story_2_1,
            StageID::Chapter2_2 => settings.story_2_2,
            StageID::Chapter2_3 => settings.story_2_3,
            StageID::Chapter2_4 => settings.story_2_4,
            StageID::Chapter2_5 => settings.story_2_5,
            StageID::Chapter3_1 => settings.story_3_1,
            StageID::Chapter3_2 => settings.story_3_2,
            StageID::Chapter3_3 => settings.story_3_3,
            StageID::Chapter3_4 => settings.story_3_4,
            StageID::Chapter3_5 => settings.story_3_5,
            StageID::Chapter3_6 => {
                watchers.split_buffer += 1;
                watchers.split_buffer.eq(&2) && settings.story_3_6
            }
            StageID::Chapter4_1 => settings.story_4_1,
            StageID::Chapter4_2 => settings.story_4_2,
            StageID::Chapter4_3 => settings.story_4_3,
            StageID::Chapter4_4 => settings.story_4_4,
            StageID::Chapter4_5 => settings.story_4_5,
            StageID::Chapter5_1 => settings.story_5_1,
            StageID::Chapter5_2 => settings.story_5_2,
            StageID::Chapter5_3 => settings.story_5_3,
            StageID::Chapter5_4 => settings.story_5_4,
            StageID::Chapter5_5 => settings.story_5_5,
            StageID::Chapter6_1 => settings.story_6_1,
            StageID::Chapter6_2 => settings.story_6_2,
            StageID::Chapter6_3 => settings.story_6_3,
            StageID::Chapter6_4 => settings.story_6_4,
            StageID::Chapter6_5 => settings.story_6_5,
            StageID::Chapter7_1 => settings.story_7_1,
            StageID::Chapter7_2 => settings.story_7_2,
            StageID::Chapter7_3 => settings.story_7_3,
            StageID::Chapter7_4 => {
                watchers.split_buffer += 1;
                watchers.split_buffer.eq(&4) && settings.story_7_4
            },
            StageID::Chapter8_1 => settings.story_8_1,
            StageID::Chapter8_2 => settings.story_8_2,
            StageID::Chapter8_3 => settings.story_8_3,
            StageID::Chapter8_4 => settings.story_8_4,
            StageID::Chapter8_5 => settings.story_8_5,
            StageID::Chapter9_1 => settings.story_9_1,
            StageID::Chapter9_2 => settings.story_9_2,
            StageID::Chapter9_3 => settings.story_9_3,
            StageID::Chapter9_4 => settings.story_9_4,
            StageID::Chapter10_1 => settings.story_10_1,
            StageID::Chapter10_2 => settings.story_10_2,
            StageID::Chapter10_3 => settings.story_10_3,
            StageID::Chapter10_4 => settings.story_10_4,
            StageID::Chapter10_5 => settings.story_10_5,
            StageID::Chapter10_6 => settings.story_10_6,
        })
}

fn reset(_watchers: &Watchers, _settings: &Settings) -> bool {
    false
}

fn is_loading(watchers: &Watchers, _settings: &Settings) -> Option<bool> {
    if watchers.stage_id.pair.is_some_and(|val| val.changed()) {
        Some(false)
    } else {
        match &watchers.is_loading.pair {
            Some(x) => match x.changed() {
                true => Some(x.current),
                _ => None,
            },
            _ => None,
        }
    }
}

fn game_time(_watchers: &Watchers, _settings: &Settings, _addresses: &Memory) -> Option<Duration> {
    None
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
enum StageID {
    Chapter1_1,
    Chapter1_2,
    Chapter1_3,
    Chapter1_4,
    Chapter2_1,
    Chapter2_2,
    Chapter2_3,
    Chapter2_4,
    Chapter2_5,
    Chapter3_1,
    Chapter3_2,
    Chapter3_3,
    Chapter3_4,
    Chapter3_5,
    Chapter3_6,
    Chapter4_1,
    Chapter4_2,
    Chapter4_3,
    Chapter4_4,
    Chapter4_5,
    Chapter5_1,
    Chapter5_2,
    Chapter5_3,
    Chapter5_4,
    Chapter5_5,
    Chapter6_1,
    Chapter6_2,
    Chapter6_3,
    Chapter6_4,
    Chapter6_5,
    Chapter7_1,
    Chapter7_2,
    Chapter7_3,
    Chapter7_4,
    Chapter8_1,
    Chapter8_2,
    Chapter8_3,
    Chapter8_4,
    Chapter8_5,
    Chapter9_1,
    Chapter9_2,
    Chapter9_3,
    Chapter9_4,
    Chapter10_1,
    Chapter10_2,
    Chapter10_3,
    Chapter10_4,
    Chapter10_5,
    Chapter10_6,
}
