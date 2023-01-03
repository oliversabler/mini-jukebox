use clap::Parser;
use crossterm::{
    execute,
    queue,
    cursor,
    terminal,
    style::{self, Stylize}, 
    Result};
use infer;
use mp3_duration;
use rodio::{Decoder, OutputStream, Sink};
use std::{fs::File, fs::metadata, time::Instant, io::Stdout};
use std::io::{BufReader, Write, stdout};
use std::path::{PathBuf, Path};
use std::time::Duration;

#[derive(Parser)]
struct Cli {
    filepath: PathBuf
}

struct Metadata {
    filepath: String,
    filename: String,
    mimetype: String,
    size: u64
}

fn get_metadata(filepath: &Path) -> Metadata {
    let filename = filepath.file_name().unwrap().to_str().unwrap();

    let file_type = infer::get_from_path(filepath)
        .expect("File read successfully.")
        .expect("Known file type.");
    
    let metadata = metadata(filepath).unwrap();
    let byte_size = metadata.len();

    Metadata {
        filepath: filepath.to_string_lossy().to_string(),
        filename: String::from(filename), 
        mimetype: String::from(file_type.mime_type()), 
        size: byte_size
    }
}

struct Jukebox {
    metadata: Metadata,
    audio_elapsed: Instant,
    audio_current: Duration,
    audio_length: Duration,
    progress_bar_position: u16,
    progress_bar_next: u16,
    progress_bar_max: u16,
    player: Stdout,
}

trait Player {
    fn new(path: &Path) -> Self;

    fn draw_metadata(&mut self);

    fn draw_progression(&mut self);

    fn play(&mut self);

    fn tick(&mut self);
}

impl Player for Jukebox {
    fn new(path: &Path) -> Self {
        let metadata = get_metadata(&path);

        // Todo: Handle different mime types
        let total_time = mp3_duration::from_path(&path).unwrap();

        Self {
            metadata,
            audio_elapsed: Instant::now(),
            audio_current: Duration::from_secs(0),
            audio_length: total_time,
            progress_bar_next: 1,
            progress_bar_position: 3,
            progress_bar_max: 10,
            player: stdout()
        }
    }

    fn draw_metadata(&mut self) {
        execute!(self.player, terminal::Clear(terminal::ClearType::All)).err();

        queue!(
            self.player,
            cursor::MoveTo(2, 1),
            style::PrintStyledContent(
                format!("Playing: {}", self.metadata.filename).yellow()))
        .unwrap();

        queue!(
            self.player,
            cursor::MoveTo(2, 2),
            style::PrintStyledContent(
                format!("Type: {}", self.metadata.mimetype).yellow()))
        .unwrap();

        queue!(
            self.player,
            cursor::MoveTo(2, 3),
            style::PrintStyledContent(
                format!("Size: {} bytes", self.metadata.size).yellow()))
        .unwrap();

        let progress_bar_start = self.progress_bar_position - self.progress_bar_next;
        queue!(
            self.player,
            cursor::MoveTo(progress_bar_start, 4),
            style::PrintStyledContent(format!("|").red()))
        .unwrap();

        let progress_bar_end = self.progress_bar_max + self.progress_bar_position;
        queue!(
            self.player,
            cursor::MoveTo(progress_bar_end, 4),
            style::PrintStyledContent(format!("|").red()))
        .unwrap();

        self.player.flush().unwrap();
    }

    fn draw_progression(&mut self) {
        queue!(
            self.player,
            cursor::MoveTo(self.progress_bar_position, 4),
            style::PrintStyledContent(format!(".").red()))
        .unwrap();

        self.progress_bar_position += 1;

        self.player.flush().unwrap();
    }

    fn play(&mut self) {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        self.draw_metadata();

        let audio = BufReader::new(File::open(&self.metadata.filepath).unwrap());

        sink.append(Decoder::new(audio).unwrap());
        sink.play();
        
        self.audio_elapsed = Instant::now();
        
        while !sink.empty() {
            self.tick();
        }

        sink.sleep_until_end();
        sink.stop();
    }

    fn tick(&mut self) {
        // This is not 100% accurate, but close enough to get the job done for now.
        self.audio_current = self.audio_elapsed.elapsed();

        let next_chunk = self.audio_length / self.progress_bar_max.into() * self.progress_bar_next.into();

        if self.audio_current > next_chunk {
            self.progress_bar_next += 1;
            self.draw_progression();
        }
    }
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let mut player: Jukebox = Player::new(&args.filepath);

    player.play();

    Ok(())
}
