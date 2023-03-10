use clipboard::{ClipboardContext, ClipboardProvider};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use prettytable::{format, row, Cell, Row, Table};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process;

#[derive(Debug, Deserialize, Serialize)]
struct Clipboard {
    data: HashMap<String, HashMap<String, String>>,
    filepath: String,
    current: String,
}

impl Clipboard {
    fn new(filepath: &str) -> Self {
        Self {
            data: HashMap::new(),
            filepath: filepath.to_string(),
            current: "default".to_string(),
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
        let history_name = Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter clipboard history name:")
            .interact()?;
        let mut clipboard_ctx: ClipboardContext = ClipboardProvider::new()?;
        let value = clipboard_ctx.get_contents()?.to_owned();
        self.data
            .entry(history_name.clone())
            .or_insert(HashMap::new())
            .insert(
                Input::<String>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter key:")
                    .interact()?,
                value,
            );
        self.save_data()?;
        println!("Data saved to clipboard history: {}", history_name);
        Ok(())
    }

    fn load(&mut self) -> Result<(), Box<dyn Error>> {
        let history_names = self.data.keys().cloned().collect::<Vec<String>>();
        let index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a clipboard history to load:")
            .items(&history_names)
            .default(0)
            .interact()?;
        self.current = history_names[index].clone();
        let options = self.data[&self.current]
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        let index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a key to load:")
            .items(&options)
            .default(0)
            .interact()?;
        let key = options[index].clone();
        let value = self.data[&self.current].get(&key).unwrap().to_owned();
        let mut clipboard_ctx: ClipboardContext = ClipboardProvider::new()?;
        clipboard_ctx.set_contents(value)?;
        println!("Data copied to clipboard.");
        Ok(())
    }

    fn list(&self) -> Result<(), Box<dyn Error>> {
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_BOX_CHARS);

        let header = Row::new(vec![
            Cell::new("History"),
            Cell::new("Key"),
            Cell::new("Value"),
        ]);
        table.add_row(header);

        for (history_name, map) in &self.data {
            table.add_row(row![history_name, "", ""]);
            for (key, value) in map {
                table.add_row(row!["", key, value]);
            }
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

        let mut matched_data = HashMap::new();
        for (history_name, inner_map) in &self.data {
            for (inner_key, inner_value) in inner_map {
                if history_name.contains(&search_term)
                    || inner_key.contains(&search_term)
                    || inner_value.contains(&search_term)
                {
                    matched_data.insert(history_name.clone(), inner_map.clone());
                    break;
                }
            }
        }

        if matched_data.is_empty() {
            println!("No results found for search term: {}", search_term);
        } else {
            let mut table = Table::new();
            table.add_row(row!["History", "Key", "Value"]);
            table.set_format(*format::consts::FORMAT_BOX_CHARS);
            for (key, inner_map) in &matched_data {
                for (inner_key, inner_value) in inner_map {
                    table.add_row(row![key, inner_key, inner_value]);
                }
            }
            table.printstd();
        }
        Ok(())
    }

    fn delete(&mut self) -> Result<(), Box<dyn Error>> {
        let history_names = self.data.keys().cloned().collect::<Vec<String>>();
        let index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a clipboard history to delete from:")
            .items(&history_names)
            .default(0)
            .interact()?;
        let history_name = history_names[index].clone();
        let options = self.data[&history_name]
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        let index = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a key to delete:")
            .items(&options)
            .default(0)
            .interact()?;
        let key = options[index].clone();
        self.data.get_mut(&history_name).unwrap().remove(&key);
        self.save_data()?;
        println!("Key deleted from clipboard history: {}", history_name);
        Ok(())
    }

    fn export(&self) -> Result<(), Box<dyn Error>> {
        let export_options = vec!["CSV", "JSON", "Exit"];
        let export_choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Export data as:")
            .items(&export_options)
            .default(0)
            .interact()?;

        match export_options[export_choice] {
            "JSON" => {
                let filename = Input::<String>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter the filename for JSON export:")
                    .interact()?;
                let file = File::create(&filename)?;
                let writer = BufWriter::new(file);
                serde_json::to_writer_pretty(writer, &self.data)?;
                println!("Clipboard data exported to {}.", filename);
            }
            "CSV" => {
                let filename = Input::<String>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter the filename for CSV export:")
                    .interact()?;

                let file = File::create(&filename)?;
                let mut writer = csv::Writer::from_writer(file);
                for (history_name, map) in &self.data {
                    for (key, value) in map {
                        writer.serialize(&[history_name, key, value])?;
                    }
                }
                writer.flush()?;
                println!("Clipboard data exported to {}.", filename);
            }
            "Exit" => {
                println!("Export cancelled.");
            }
            _ => return Err("Unsupported format".into()),
        }

        Ok(())
    }

    fn import(&mut self) -> Result<(), Box<dyn Error>> {
        let import_options = vec!["CSV", "JSON", "Exit"];
        let import_choice = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Import data from:")
            .items(&import_options)
            .default(0)
            .interact()?;

        match import_options[import_choice] {
            "JSON" => {
                let filename = Input::<String>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter the filename for JSON import:")
                    .interact()?;
                let file_content = fs::read_to_string(&filename)?;
                let imported_data: HashMap<String, HashMap<String, String>> =
                    serde_json::from_str(&file_content)?;
                self.merge_data(imported_data)?;
                println!("Data imported from {}.", filename);
            }
            "CSV" => {
                let filename = Input::<String>::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter the filename for CSV import:")
                    .interact()?;

                let file = File::open(&filename)?;
                let mut reader = csv::Reader::from_reader(file);
                let mut imported_data = HashMap::new();
                for result in reader.deserialize() {
                    let record: Vec<String> = result?;
                    let history_name = record[0].clone();
                    let key = record[1].clone();
                    let value = record[2].clone();
                    imported_data
                        .entry(history_name)
                        .or_insert(HashMap::new())
                        .insert(key, value);
                }
                self.merge_data(imported_data)?;
                println!("Data imported from {}.", filename);
            }
            "Exit" => {
                println!("Import cancelled.");
            }
            _ => return Err("Unsupported format".into()),
        }

        Ok(())
    }

    fn merge_data(
        &mut self,
        imported_data: HashMap<String, HashMap<String, String>>,
    ) -> Result<(), Box<dyn Error>> {
        for (history_name, map) in imported_data {
            let existing_map = self
                .data
                .entry(history_name.clone())
                .or_insert(HashMap::new());
            
            for (key, value) in map {
                existing_map.insert(key, value);
            }
        }
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
            5 => clipboard.export()?,
            6 => clipboard.import()?,
            7 => {
                clipboard.save_data()?;
                println!("Data saved before quitting.");
                process::exit(0);
            }
            _ => unreachable!(),
        }
    }
}
