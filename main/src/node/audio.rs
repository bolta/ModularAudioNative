use std::sync::{mpsc, Mutex};
use crate::common::util::ignore_errors;
use crate::core::{
    common::*,
    common::Sample,
    context::*,
    machine::*,
    node::*,
};
use cpal::{
    traits::{
        DeviceTrait,
        HostTrait,
        StreamTrait,
    },
    FromSample, /* Sample, */ SizedSample, SupportedStreamConfig,
};
use node_macro::node_impl;
// // TODO 全体的に要整理
// use portaudio as pa;
const FRAMES: u32 = 1000;
const INTERLEAVED: bool = true;
const QUEUE_LENGTH: usize = 100000;
// // #4 マルチマシン対応
// // PortAudio の Stream は生ポインタを持っている都合で Send にならないので、
// // ムリヤリ Send にするためのラッパーをかます。
// // https://users.rust-lang.org/t/workaround-missing-send-trait-for-the-ffi/30828/7
// // ノード生成時には初期化含めて一切触らず、別スレッドでの演奏開始時に初めて初期化するので問題はないはず
// struct SendWrapper(pa::Stream<pa::Blocking<pa::stream::Buffer>, pa::Output<Sample>>);
// unsafe impl Send for SendWrapper { }
// pub struct PortAudioOut {
//  base_: NodeBase,
//  input: ChanneledNodeIndex,
//  stream: Option<SendWrapper>,
//  buffer: Vec<Sample>,
//  buffer_size: usize,
// }
// impl PortAudioOut {
//  pub fn new(base: NodeBase, input: ChanneledNodeIndex) -> Self {
//      let channels = input.channels();
//      let buffer_size = FRAMES as usize * channels as usize;
//      Self {
//          base_: base,
//          input,
//          stream: None,
//          buffer: Vec::with_capacity(buffer_size),
//          buffer_size,
//      }
//  }
// }
// #[node_impl]
// impl Node for PortAudioOut {
//  // ノードグラフ上で出力するチャンネル数は 0
//  fn channels(&self) -> i32 { 0 }
//  fn activeness(&self) -> Activeness { Activeness::Active } // TODO でいいのかな
//  // TODO ↓これ抽象クラス的なものに括り出したい
//  fn initialize(&mut self, context: &Context, _env: &mut Environment) {
//      let pa = pa::PortAudio::new().expect("error");
//      // let default_host = pa.default_host_api().expect("error");
//      // println!("default host: {:#?}", pa.host_api_info(default_host));
//      let output_device = pa.default_output_device().expect("error");
//      let output_info = pa.device_info(output_device).expect("error");
//      // println!("Use output device info: {:#?}", &output_info);
//      // 出力の設定
//      let latency = output_info.default_low_output_latency;
//      // float32形式で再生
//      let output_params =
//          pa::StreamParameters::<f32>::new(output_device, self.input.channels(), INTERLEAVED, latency);
//      let sample_rate = context.sample_rate() as f64;
//      pa.is_output_format_supported(output_params, sample_rate).expect("error");
//      let output_settings = pa::OutputStreamSettings::new(output_params, sample_rate as f64, FRAMES);
//      let stream = pa.open_blocking_stream(output_settings).expect("error");
//      self.stream = Some(SendWrapper(stream));
//      match &mut self.stream {
//          None => { }
//          Some(stream) => stream.0.start().expect("error")
//      }
//  }
//  fn upstreams(&self) -> Upstreams { vec![self.input] }
//  fn execute(&mut self, _inputs: &Vec<Sample>, _output: &mut [OutputBuffer], _context: &Context, _env: &mut Environment) {
//      if self.buffer.len() < self.buffer_size { return; }
//      let b = &mut self.buffer;
//      match &mut self.stream {
//          None => { }
//          Some(stream) => {
//              stream.0.write(FRAMES as u32, |output| {
//                  for (i, sample) in b.iter().enumerate() {
//                      output[i] = 0.5 * sample;
//                  };
//              }).expect("error");
//          }
//      }
//  }
//  fn update(&mut self, inputs: &Vec<Sample>, _context: &Context, _env: &mut Environment) {
//      if self.buffer.len() >= self.buffer_size { self.buffer.clear(); }
//      for ch in 0 .. self.input.channels() {
//          self.buffer.push(inputs[ch as usize]);
//      }
//  }
//  fn finalize(&mut self, _context: &Context, _env: &mut Environment) {
//      match &mut self.stream {
//          None => { }
//          Some(stream) => {
//              ignore_errors(stream.0.stop());
//              ignore_errors(stream.0.close());
//          }
//      }
//      self.stream = None;
//  }
// }
pub struct AudioOut {
    base_: NodeBase,
    input: ChanneledNodeIndex,
    // stream: Option<SendWrapper>,
    buffer: Vec<Sample>,
    buffer_size: usize,
    sender: Option<mpsc::SyncSender<Sample>>,
    receiver: Option<mpsc::Receiver<Sample>>,
    initialized: bool,
}
impl AudioOut {
    pub fn new(base: NodeBase, input: ChanneledNodeIndex) -> Self {
        let channels = input.channels();
        let buffer_size = FRAMES as usize * channels as usize;
        Self {
            base_: base,
            input,
            // stream: None,
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
            sender: None,
            receiver: None,
            initialized: false,
        }
    }
}
#[node_impl]
impl Node for AudioOut {
    // ノードグラフ上で出力するチャンネル数は 0
    fn channels(&self) -> i32 { 0 }
    fn activeness(&self) -> Activeness { Activeness::Active } // TODO でいいのかな
    // TODO ↓これ抽象クラス的なものに括り出したい
    fn initialize(&mut self, context: &Context, _env: &mut Environment) {
        let (sender, receiver) = mpsc::sync_channel::<f32>(self.input.channels() as usize * QUEUE_LENGTH);
        self.sender = Some(sender);
        self.receiver = Some(receiver);
        let host = cpal::default_host();
        let device = host.default_output_device()
        .expect("failed to find output device");
    
        let config_def = device.default_output_config().unwrap();
        let channels = 1u16; // self.input.channels() as u16;
        let config = SupportedStreamConfig::new(
            channels,
            cpal::SampleRate(context.sample_rate() as u32),
            config_def.buffer_size().clone(),
            cpal::SampleFormat::F32,
        );
        // let sample_rate = context.sample_rate() as f32;
        // Produce a sinusoid of maximum amplitude.
        // let mut sample_clock = 0f32;
        // let mut next_value = move || {
        //     sample_clock = (sample_clock + 1.0) % sample_rate;
        //     2f32 * (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()
        // };
        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    
        // let r = &receiver as * const mpsc::Receiver<f32>;
        // let r = Mutex::new(&receiver);
        // let receiver_ptr = self.receiver.as_ref().unwrap() as *const mpsc::Receiver<f32>;
        // let receiver_sync = unsafe {
        //     fn discard_lifetime<'a, T>(r: &'a T) -> &'static T {
        //         unsafe { &* (r as *const T) }
        //     }
        //     force_send_sync::SendSync::new(discard_lifetime(self.receiver.as_ref().unwrap()))
        // };
        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // write_data(data, channels as usize, &mut next_value)
                // let r: &mpsc::Receiver<f32> = unsafe { &* (r as *const mpsc::Receiver<f32>) };
                // let r = unsafe { &*r };
                // let re = r.lock().unwrap();
                dbg!("callback");
                dbg!(std::thread::current().id());
                for i in 0 .. data.len() {
                    // TODO 遅そう…
                    dbg!("recv");
                    data[i] = receiver_sync.recv().unwrap_or(0.12345f32);
                    dbg!(data[i]);
                }
            },
            err_fn,
            None,
        ).expect("hoge");
        dbg!("play");
        stream.play().expect("boke");
        // TODO ここでしばらく待つと、コールバックは呼ばれるが execute() が呼ばれないので、recv() で永遠にブロックしてしまう。
        // 待たないと、コールバックが呼ばれず execute() だけが呼ばれ、mpsc のキューが一杯になってブロックしてしまう。
        // なぜ両方呼ばれないのか、なぜウェイトの有無で呼ばれる方が変わるのか、どちらも不明
        // 両者は別々のスレッドで動くので（確認済）、execute で send してコールバックで recv することを並行でできるはずなのだが…
        // コールバックに receiver を渡すときにかなり汚いことをしているが、
        // コールバックのスレッドは常に一定なので便宜上 Sync をつけて渡すことは問題ないはずだし、
        // sender/receiver は self と同じ寿命を持ち演奏中は必ず存在するので寿命をもみ消すことも問題はないはずだが…

        // std::thread::sleep_ms(5000);
        // self.initialized = true;
    }
    fn upstreams(&self) -> Upstreams { vec![self.input] }
    fn execute(&mut self, inputs: &Vec<Sample>, _output: &mut [OutputBuffer], context: &Context, _env: &mut Environment) {
        // if self.buffer.len() < self.buffer_size { return; }
        // let b = &mut self.buffer;
        // match &mut self.stream {
        //  None => { }
        //  Some(stream) => {
        //      stream.0.write(FRAMES as u32, |output| {
        //          for (i, sample) in b.iter().enumerate() {
        //              output[i] = 0.5 * sample;
        //          };
        //      }).expect("error");
        //  }
        // }
        let sender = self.sender.as_mut().unwrap();
        static mut COUNT: i32 = 0;
        inputs.iter().for_each(|sample| {
            let res = sender.send(*sample);
            dbg!(unsafe { COUNT }, sample);
            dbg!(std::thread::current().id());
            // dbg!(*sample);
            unsafe { COUNT += 1; }
        });
        // if self.initialized { return; }
    }
    // fn update(&mut self, inputs: &Vec<Sample>, _context: &Context, _env: &mut Environment) {
    //  if self.buffer.len() >= self.buffer_size { self.buffer.clear(); }
    //  for ch in 0 .. self.input.channels() {
    //      self.buffer.push(inputs[ch as usize]);
    //  }
    // }
    fn finalize(&mut self, _context: &Context, _env: &mut Environment) {
        // match &mut self.stream {
        //  None => { }
        //  Some(stream) => {
        //      ignore_errors(stream.0.stop());
        //      ignore_errors(stream.0.close());
        //  }
        // }
        // self.stream = None;
    }
}

fn write_data<T>(receiver: force_send_sync::SendSync<&mpsc::Receiver<Sample>>) -> FnMut (&mut [T], &cpal::OutputCallbackInfo)
where T: cpal::Sample + FromSample<f32> {
	// let receiver_sync = force_send_sync::SendSync::new(receiver);

	move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
		// write_data(data, channels as usize, &mut next_value)
		// let r: &mpsc::Receiver<f32> = unsafe { &* (r as *const mpsc::Receiver<f32>) };
		// let r = unsafe { &*r };
		// let re = r.lock().unwrap();
		dbg!("callback");
		dbg!(std::thread::current().id());
		for i in 0 .. data.len() {
			// TODO 遅そう…
			dbg!("recv");
			data[i] = T::from_sample(receiver.recv().unwrap_or(0.12345f32));
			dbg!(data[i]);
		}
	}
}
