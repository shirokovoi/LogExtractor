use anyhow::{anyhow, Context, Result};
use clap::Parser;
use flate2::bufread::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{copy, BufRead, BufReader, BufWriter, Write};

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use super::{decompress_into, sort_files};
    use anyhow::Result;

    #[test]
    fn sort_inputs_test() -> Result<()> {
        let inputs = vec![
            String::from("a.log.4.gz"),
            String::from("a.log.1.gz"),
            String::from("a.log.30.gz"),
            String::from("a.log.2.gz"),
        ];
        let expected = vec![
            String::from("a.log.1.gz"),
            String::from("a.log.2.gz"),
            String::from("a.log.4.gz"),
            String::from("a.log.30.gz"),
        ];

        assert_eq!(expected, sort_files(&inputs)?);
        Ok(())
    }

    #[test]
    fn decompress_test() -> Result<()> {
        let buffer = [
            0x1f, 0x8b, 0x8, 0x8, 0x60, 0x6d, 0xd8, 0x62, 0x0, 0x3, 0x69, 0x6e, 0x2e, 0x74, 0x78,
            0x74, 0x0, 0xf3, 0x48, 0xcd, 0xc9, 0xc9, 0x57, 0x8, 0xcf, 0x2f, 0xca, 0x49, 0xe1, 0x2,
            0x0, 0xe3, 0xe5, 0x95, 0xb0, 0xc, 0x0, 0x0, 0x0,
        ];

        let reader = Cursor::new(&buffer);
        let mut writer_buf: Vec<u8> = Vec::new();
        let mut writer = Cursor::new(&mut writer_buf);

        decompress_into(reader, &mut writer)?;

        writer.flush()?;
        let got = String::from_utf8(writer_buf)?;
        let expected = String::from("Hello World\n");

        assert_eq!(got, expected);

        Ok(())
    }
}

#[derive(Parser, Debug)]
struct ProgramArgs {
    #[clap(short, long)]
    output_file: String,
    input_files: Vec<String>,
}

fn decompress_into<R: BufRead, W: Write>(reader: R, writer: &mut W) -> Result<()> {
    let mut decoder = GzDecoder::new(reader);
    copy(&mut decoder, writer)?;

    Ok(())
}

fn sort_files(files: &Vec<String>) -> Result<Vec<String>> {
    let result: Result<BTreeMap<u32, String>> = files
        .iter()
        .map(|item| {
            let number = item
                .rsplit(".")
                .skip(1)
                .next()
                .ok_or(anyhow!("Wrong filename format! ({})", item))?
                .parse::<u32>()?;

            Ok((number, item.to_owned()))
        })
        .collect();

    Ok(result?.into_iter().map(|(_, value)| value).collect())
}

fn main() -> Result<()> {
    let args = ProgramArgs::parse();
    let sorted = sort_files(&args.input_files)?;

    let output_file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .open(args.output_file)?;

    let mut writer = BufWriter::new(output_file);

    let bar = ProgressBar::new(sorted.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .progress_chars("##-"),
    );

    for filepath in &sorted {
        bar.set_message(format!("Process {}", &filepath));
        bar.inc(1);

        let file = File::open(filepath)
            .with_context(|| format!("Failed to open archive file ({})", filepath))?;
        let reader = BufReader::new(file);

        decompress_into(reader, &mut writer)?;
    }

    bar.finish();

    Ok(())
}
