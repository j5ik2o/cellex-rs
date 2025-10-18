#![cfg_attr(all(target_arch = "arm", target_os = "none"), no_std)]
#![cfg_attr(all(target_arch = "arm", target_os = "none"), no_main)]

#[cfg(all(target_arch = "arm", target_os = "none"))]
mod hw {
  extern crate alloc;

  use alloc::rc::Rc;
  use core::cell::RefCell;

  use alloc_cortex_m::CortexMHeap;
  use cellex_actor_core_rs::{ActorSystem, Behaviors, Props};
  use cellex_actor_embedded_rs::LocalMailboxRuntime;
  use cortex_m::{asm, interrupt};
  use cortex_m_rt::entry;
  use embedded_hal::digital::v2::OutputPin;
  use panic_halt as _;
  use rp2040_boot2;
  use rp2040_hal::{
    self as hal,
    clocks::Clock,
    gpio::{bank0::Gpio25, FunctionSioOutput, PullDown},
    pac,
    sio::Sio,
    watchdog::Watchdog,
  };

  const HEAP_SIZE: usize = 16 * 1024;
  static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

  #[global_allocator]
  static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

  #[link_section = ".boot2"]
  #[used]
  static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

  #[derive(Clone, Copy, Debug)]
  enum Command {
    Greet(&'static str),
    Report,
    Stop,
  }

  #[entry]
  fn main() -> ! {
    let heap_start = core::ptr::addr_of_mut!(HEAP) as usize;
    interrupt::free(|_| unsafe {
      ALLOCATOR.init(heap_start, HEAP_SIZE);
    });

    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
      12_000_000,
      pac.XOSC,
      pac.CLOCKS,
      pac.PLL_SYS,
      pac.PLL_USB,
      &mut pac.RESETS,
      &mut watchdog,
    )
    .ok()
    .unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, &mut pac.RESETS);

    let led_pin: LedHandle =
      Rc::new(RefCell::new(pins.gpio25.into_function::<FunctionSioOutput>().into_push_pull_output()));
    let system_clock_hz = clocks.system_clock.freq().to_Hz();

    let mut system = ActorSystem::new(LocalMailboxRuntime::default());
    let mut root = system.root_context();

    let behavior_led = led_pin.clone();
    let behavior_clock_hz = system_clock_hz;

    let greeter_props = Props::with_behavior(move || {
      let setup_led = behavior_led.clone();
      let clock_hz = behavior_clock_hz;
      Behaviors::setup(move |ctx| {
        let ready_blinks = ((ctx.actor_id().0 as u8) % 3 + 1) as usize;
        blink_repeated(&setup_led, ready_blinks, 60, 60, clock_hz);

        let message_led = setup_led.clone();
        let mut greeted = 0usize;
        Ok(Behaviors::receive_message(move |msg: Command| match msg {
          | Command::Greet(name) => {
            blink_message(&message_led, &BlinkPattern::short_pulse(), clock_hz);
            greeted = greeted.wrapping_add(1);
            // 名前の長さで追加の点滅パターンを作成し、誰に挨拶したかを伝える。
            let name_len = name.len().min(5) as u8;
            if name_len > 0 {
              blink_repeated(&message_led, name_len as usize, 80, 80, clock_hz);
            }
            Ok(Behaviors::same())
          },
          | Command::Report => {
            let flashes = core::cmp::max(greeted, 1);
            blink_repeated(&message_led, flashes, 200, 120, clock_hz);
            Ok(Behaviors::same())
          },
          | Command::Stop => {
            blink_message(&message_led, &BlinkPattern::shutdown(), clock_hz);
            Ok(Behaviors::stopped())
          },
        }))
      })
    });

    let greeter = root.spawn(greeter_props).expect("spawn greeter");
    greeter.tell(Command::Greet("Alice")).expect("greet Alice");
    greeter.tell(Command::Greet("Bob")).expect("greet Bob");
    greeter.tell(Command::Report).expect("report greetings");
    greeter.tell(Command::Stop).expect("stop greeter");

    system.run_until_idle().expect("drain mailbox");

    set_led_state(&led_pin, false);

    // ループに入って省電力待機。LEDは消灯状態を維持する。
    loop {
      asm::wfi();
    }
  }

  struct BlinkPattern {
    on_ms:  u32,
    off_ms: u32,
    repeat: usize,
  }

  impl BlinkPattern {
    const fn new(on_ms: u32, off_ms: u32, repeat: usize) -> Self {
      Self { on_ms, off_ms, repeat }
    }

    const fn short_pulse() -> Self {
      Self::new(120, 80, 1)
    }

    const fn shutdown() -> Self {
      Self::new(300, 200, 3)
    }
  }

  fn blink_message(led: &LedHandle, pattern: &BlinkPattern, sys_hz: u32) {
    blink_repeated(led, pattern.repeat, pattern.on_ms, pattern.off_ms, sys_hz);
  }

  fn blink_repeated(led: &LedHandle, times: usize, on_ms: u32, off_ms: u32, sys_hz: u32) {
    for _ in 0..times {
      set_led_state(led, true);
      delay_ms(on_ms, sys_hz);
      set_led_state(led, false);
      delay_ms(off_ms, sys_hz);
    }
  }

  fn set_led_state(led: &LedHandle, high: bool) {
    let mut led_guard = led.borrow_mut();
    if high {
      led_guard.set_high().ok();
    } else {
      led_guard.set_low().ok();
    }
  }

  fn delay_ms(ms: u32, sys_hz: u32) {
    if ms == 0 {
      return;
    }
    let cycles = (sys_hz / 1_000).saturating_mul(ms.max(1));
    asm::delay(cycles);
  }

  type LedPin = hal::gpio::Pin<Gpio25, FunctionSioOutput, PullDown>;
  type LedHandle = Rc<RefCell<LedPin>>;
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
use hw::*;

#[cfg(not(all(target_arch = "arm", target_os = "none")))]
fn main() {
  println!(
    "RP2040 Behaviors Greeter exampleはthumbv6m-none-eabiターゲット向けです。\n\
     `cargo run --release --example rp2040_behaviors_greeter --target thumbv6m-none-eabi` でビルドしてください。"
  );
}
