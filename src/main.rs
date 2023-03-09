use clipboard::{ClipboardContext, ClipboardProvider};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use prettytable::{format, row, Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process;

#[derive(Debug, Deserialize, Serialize)]
struct Clipboard {
    data: HashMap<String, String>,
    filepath: String,
}

impl Clipboard {
    fn new(filepath: &str) -> Self {
        Self {
            data: HashMap::new(),
            filepath: filepath.to_string(),
        }
    }

    fn load_data(&mut self) -> Result<(), Box<dyn Error>> {
        let file_content = fs::read_to_string(&self.filepath)?;
        self.data = serde_json::from_str(&file_content)?;
        Ok(())
    }

    fn save_data(&mut self) -> Result<(), Box<dyn Error>> {
        let data_str = serde_json::to_string_pretty(&self.data)?;
        let mut file = fs::File::create(&self.filepath)?;
        file.write_all(data_str.as_bytes())?;
        Ok(())
    }

    fn save(&mut self) -> Result<(), Box<dyn Error>> {
        let key = Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter key:")
            .interact()?;
        let mut clipboard_ctx: ClipboardContext = ClipboardProvider::new()?;
        let value = clipboard_ctx.get_contents()?.to_owned();
        self.data.insert(key, value);
        self.save_data()?;
        println!("Data saved!");
        Ok(())
    }

    fn load(&mut self) -> Result<(), Box<dyn Error>> {
        let options = self.data.keys().cloned().collect::<Vec<String>>();
        let index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a key to load:")
            .items(&options)
            .default(0)
            .interact()?;
        let key = options[index].clone();
        let value = self.data.get(&key).unwrap().to_owned();
        let mut clipboard_ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        clipboard_ctx.set_contents(value)?;
        println!("Data copied to clipboard.");
        Ok(())
    }

    fn list(&self) -> Result<(), Box<dyn Error>> {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_BOX_CHARS);

        let header = Row::new(vec![Cell::new("Key"), Cell::new("Value")]);
        table.add_row(header);

        for (key, value) in &self.data {
            let row = Row::new(vec![Cell::new(key), Cell::new(value)]);
            table.add_row(row);
        }

        table.printstd();
        Ok(())
    }

    fn search(&mut self) -> Result<(), Box<dyn Error>> {
        let search_term = Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter a search term:")
            .validate_with(|input: &String| {
                if input.is_empty() {
                    Err("Search term cannot be empty.".to_string())
                } else {
                    Ok(())
                }
            })
            .interact()?;

        let matched_data = self
            .data
            .iter()
            .filter(|(key, value)| key.contains(&search_term) || value.contains(&search_term))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<HashMap<_, _>>();

        if matched_data.is_empty() {
            println!("No matching data found.");
        } else {
            let mut table = Table::new();
            table.add_row(row!["Key", "Value"]);
            table.set_format(*format::consts::FORMAT_BOX_CHARS);
            for (key, value) in &matched_data {
                table.add_row(row![key, value]);
            }
            table.printstd();
        }
        Ok(())
    }

    fn delete(&mut self) -> Result<(), Box<dyn Error>> {
        let options = self.data.keys().cloned().collect::<Vec<String>>();
        let index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a key to delete:")
            .items(&options)
            .default(0)
            .interact()?;
        let key = options[index].clone();
        self.data.remove(&key);
        self.save_data()?;
        println!("Data deleted!");
        Ok(())
    }

    fn export(&self, format: &str) -> Result<(), Box<dyn Error>> {
        let file = fs::File::create(format!("clipboard.{}", format))?;

        match format {
            "json" => serde_json::to_writer_pretty(file, &self.data)?,
            "csv" => {
                let mut writer = csv::Writer::from_writer(file);
                for (key, value) in &self.data {
                    writer.write_record(&[key, value])?;
                }
                writer.flush()?;
            }
            _ => return Err("Unsupported format".into()),
        }

        println!("Data exported as {}!", format.to_uppercase());
        Ok(())
    }

    fn import(&mut self, format: &str, filepath: &str) -> Result<(), Box<dyn Error>> {
        let file = fs::File::open(filepath)?;
        match format {
            "json" => {
                let data: HashMap<String, String> = serde_json::from_reader(file)?;
                self.data.extend(data);
            }
            "csv" => {
                let mut reader = csv::Reader::from_reader(file);
                for result in reader.records() {
                    let record = result?;
                    self.data.insert(record[0].to_owned(), record[1].to_owned());
                }
            }
            _ => return Err("Unsupported format".into()),
        }
        println!("Data imported from {}!", format.to_uppercase());
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let clipboard_file = Path::new("clipboard.json");

    let mut clipboard = if clipboard_file.exists() {
        let mut clipboard = Clipboard::new("clipboard.json");
        clipboard.load_data()?;
        clipboard
    } else {
        Clipboard::new("clipboard.json")
    };

    let choices = vec![
        "Save", "Load", "List", "Search", "Delete", "Export", "Import", "Quit",
    ];
    loop {
        let choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an action:")
            .items(&choices)
            .default(0)
            .interact()?;
        match choice {
            0 => clipboard.save()?,
            1 => clipboard.load()?,
            2 => clipboard.list()?,
            3 => clipboard.search()?,
            4 => clipboard.delete()?,
            5 => clipboard.export("json")?,
            6 => {
                let format_choices = vec!["json", "csv"];
                let format = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Select a format:")
                    .items(&format_choices)
                    .default(0)
                    .interact()?;
                let filepath = Input::<String>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter file path:")
                    .interact()?;
                clipboard.import(format_choices[format], &filepath)?;
            }
            7 => {
                clipboard.save_data()?;
                println!("Data saved before quitting.");
                process::exit(0);
            }
            _ => unreachable!(),
        }
    }
}
