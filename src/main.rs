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

struct Jukebox {
    metadata: Metadata,
    audio_elapsed: Instant,
    audio_current: Duration,
    audio_length: Duration,
    progress_bar_position: u16,
    progress_bar_max: u16,
    player: Stdout,
}

trait Player {
    fn new(filepath: &Path) -> Self;

    fn draw_metadata(&mut self);

    fn draw_progression(&mut self);

    fn play(&mut self);

    fn tick(&mut self);
}

impl Player for Jukebox {
    fn new(filepath: &Path) -> Self {
        let file_type = infer::get_from_path(filepath)
            .expect("File read successfully.")
            .expect("Known file type.");

        let metadata = metadata(filepath).unwrap();

        let metadata = Metadata {
            filepath: filepath.to_str().unwrap().to_string(),
            filename: String::from(filepath.file_name().unwrap().to_str().unwrap()),
            mimetype: String::from(file_type.mime_type()),
            size: metadata.len()
        };

        // Todo: Handle different mime types
        let total_time = mp3_duration::from_path(&filepath).unwrap();

        Self {
            metadata,
            audio_elapsed: Instant::now(),
            audio_current: Duration::from_secs(0),
            audio_length: total_time,
            progress_bar_position: 1,
            progress_bar_max: 20,
            player: stdout()
        }
    }

    fn draw_metadata(&mut self) {
        execute!(self.player, terminal::Clear(terminal::ClearType::All)).err();

        let audio_info_types = vec!["Playing:", "Type:", "Size:"];
        for (i, info_type) in audio_info_types.iter().enumerate() {
            queue!(
                self.player,
                cursor::MoveTo(0, i as u16),
                style::PrintStyledContent(info_type.grey())
            ).unwrap();
        };
        
        let col_width = 9;

        queue!(
            self.player,
            cursor::MoveTo(col_width, 0),
            style::PrintStyledContent(format!("{}", self.metadata.filename).green()),
            cursor::MoveTo(col_width, 1),
            style::PrintStyledContent(format!("{}", self.metadata.mimetype).green()),
            cursor::MoveTo(col_width, 2),
            style::PrintStyledContent(format!("{} bytes", self.metadata.size).green()),
            cursor::MoveTo(0, 3),
            style::PrintStyledContent(format!("[").grey()),
            cursor::MoveTo(self.progress_bar_max + 1, 3),
            style::PrintStyledContent(format!("]").grey())
        ).unwrap();

        self.player.flush().unwrap();
    }

    fn draw_progression(&mut self) {
        queue!(
            self.player,
            cursor::MoveTo(self.progress_bar_position, 3),
            style::PrintStyledContent(format!("=").green().bold())
        ).unwrap();

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

        let next_chunk = self.audio_length / self.progress_bar_max as u32 * self.progress_bar_position as u32;

        if self.audio_current > next_chunk {
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
