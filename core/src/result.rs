use std::time::Duration;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct StreamResult {
    pub elapsed: Duration,
    pub pktcount: u64,
    pub bytes: u64,
    pub bytes_expected: u64,
    pub testdone: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct ClientResult {
    pub total: StreamResult,
    pub streams: Vec<StreamResult>,
}


impl StreamResult {
    pub fn get_througtput(&self) -> u64 {
        let elapsed = self.elapsed.as_micros() as u64;
        if elapsed == 0 {
            return 0;
        }
        8 * ((1000000 * self.bytes) / elapsed)
    }

    pub fn throughput_is(&self, value: u64, percent_error: u64) -> bool {
        let througput = self.get_througtput();
        let delta = (value * percent_error) / 100;
        let min = value - delta;
        let max = value + delta;

        througput >= min && througput <= max
    }
    /*
    pub fn throughput_is_nominal(&self, percent_error: u64) -> bool {
        let expected = bytes_expected / self.duration_expected;
        self.throughput_is(expected, percent_error)
    }
    */

    pub fn throughput_lower_than(&self, value: u64) -> bool {
        self.get_througtput() < value
    }
}

impl ClientResult {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn add_stream_result(&mut self, result: &StreamResult) {
        if self.total.elapsed < result.elapsed {
            self.total.elapsed = result.elapsed;
        }
        self.total.pktcount += result.pktcount;
        self.total.bytes += result.bytes;
        self.total.bytes_expected += result.bytes_expected;
        self.total.testdone |= result.testdone;
        self.streams.push(result.clone());
    }

    pub fn print_summary(&self) {
        println!("[{}] Sum: {} kbps, Rx: {}kB, Expected: {}kB",
            self.total.elapsed.as_secs(),
            self.total.get_througtput()/1000,
            self.total.bytes/1000,
            self.total.bytes_expected/1000);
    }

    pub fn pretty_print(&self) {
        self.print_summary();
        if self.streams.len() <= 1 {
            return;
        }

        let mut i = 0;
        for stream in &self.streams {
            println!("  - Stream {}: {} kbps, Pkt: {}, Expected: {}",
                i,
                stream.get_througtput()/1000,
                stream.pktcount,
                stream.bytes_expected);
            i += 1;
        }
    }

    pub fn total(&self) -> &StreamResult {
        &self.total
    }

    pub fn collect_and_override(streams: &Vec<Mutex<StreamResult>>, elapsed: Duration, bytes_expected: u64) -> Self {
        let mut client = ClientResult::default();
        for stream in streams {
            let mut stream = stream.lock().unwrap().clone();
            stream.elapsed = elapsed;
            stream.bytes_expected = bytes_expected as u64;
            client.add_stream_result(&stream);
        }
        client
    }

    pub fn collect(streams: &Vec<Mutex<StreamResult>>) -> Self {
        let mut client = ClientResult::default();
        for stream in streams {
            let stream = stream.lock().unwrap().clone();
            client.add_stream_result(&stream);
        }
        client
    }
}