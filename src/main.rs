// Copyright (c) 2023 Jan Holthuis <jan.holthuis@rub.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use binrw::{BinRead, ReadOptions};
use clap::{Parser, Subcommand};
use rekordcrate::anlz::ANLZ;
use rekordcrate::pdb::{Header, PageType, Row};
use rekordcrate::setting::Setting;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List the playlist tree from a Pioneer Database (`.PDB`) file.
    ListPlaylists {
        /// File to parse.
        #[arg(value_name = "PDB_FILE")]
        path: PathBuf,
    },
    /// Parse and dump a Rekordbox Analysis (`ANLZXXXX.DAT`) file.
    DumpANLZ {
        /// File to parse.
        #[arg(value_name = "ANLZ_FILE")]
        path: PathBuf,
    },
    /// Parse and dump a Pioneer Database (`.PDB`) file.
    DumpPDB {
        /// File to parse.
        #[arg(value_name = "PDB_FILE")]
        path: PathBuf,
    },
    /// Read a Pioneer Database (`.PDB`) file and write the serialization to a different place.
    ReexportPDB {
        /// File to parse.
        #[arg(value_name = "PDB_IN_FILE")]
        inpath: PathBuf,
        /// File to write.
        #[arg(value_name = "PDB_OUT_FILE")]
        outpath: PathBuf,
    },
    /// Parse and dump a Pioneer Settings (`*SETTING.DAT`) file.
    DumpSetting {
        /// File to parse.
        #[arg(value_name = "SETTING_FILE")]
        path: PathBuf,
    },
}

fn list_playlists(path: &PathBuf) -> rekordcrate::Result<()> {
    use rekordcrate::pdb::{PlaylistTreeNode, PlaylistTreeNodeId};
    use std::collections::HashMap;

    fn print_children_of(
        tree: &HashMap<PlaylistTreeNodeId, Vec<PlaylistTreeNode>>,
        id: PlaylistTreeNodeId,
        level: usize,
    ) {
        tree.get(&id)
            .iter()
            .flat_map(|nodes| nodes.iter())
            .for_each(|node| {
                println!(
                    "{}{} {}",
                    "    ".repeat(level),
                    if node.is_folder() { "🗀" } else { "🗎" },
                    node.name.clone().into_string().unwrap(),
                );
                print_children_of(tree, node.id, level + 1);
            });
    }

    let mut reader = std::fs::File::open(path)?;
    let header = Header::read(&mut reader)?;

    let mut tree: HashMap<PlaylistTreeNodeId, Vec<PlaylistTreeNode>> = HashMap::new();

    header
        .tables
        .iter()
        .filter(|table| table.page_type == PageType::PlaylistTree)
        .flat_map(|table| {
            header
                .read_pages(
                    &mut reader,
                    &ReadOptions::new(binrw::Endian::NATIVE),
                    (&table.first_page, &table.last_page),
                )
                .unwrap()
                .into_iter()
                .flat_map(|page| page.row_groups.into_iter())
                .flat_map(|row_group| {
                    row_group
                        .present_rows()
                        .map(|row| {
                            if let Row::PlaylistTreeNode(playlist_tree) = row {
                                playlist_tree
                            } else {
                                unreachable!("encountered non-playlist tree row in playlist table");
                            }
                        })
                        .cloned()
                        .collect::<Vec<PlaylistTreeNode>>()
                        .into_iter()
                })
        })
        .for_each(|row| tree.entry(row.parent_id).or_default().push(row));

    print_children_of(&tree, PlaylistTreeNodeId(0), 0);

    Ok(())
}

fn dump_anlz(path: &PathBuf) -> rekordcrate::Result<()> {
    let mut reader = std::fs::File::open(path)?;
    let anlz = ANLZ::read(&mut reader)?;
    println!("{:#?}", anlz);

    Ok(())
}

fn dump_pdb(path: &PathBuf) -> rekordcrate::Result<()> {
    let mut reader = std::fs::File::open(path)?;
    let header = Header::read(&mut reader)?;

    println!("{:#?}", header);

    for (i, table) in header.tables.iter().enumerate() {
        println!("Table {}: {:?}", i, table.page_type);
        for page in header
            .read_pages(
                &mut reader,
                &ReadOptions::new(binrw::Endian::NATIVE),
                (&table.first_page, &table.last_page),
            )
            .unwrap()
            .into_iter()
        {
            println!("  {:?}", page);
            page.row_groups.iter().for_each(|row_group| {
                println!("    {:?}", row_group);
                for row in row_group.present_rows() {
                    println!("      {:?}", row);
                }
            })
        }
    }

    Ok(())
}

fn reexport_pdb(inpath: &PathBuf, outpath: &PathBuf) -> rekordcrate::Result<()> {
    use binrw::BinWrite;
    use binrw::WriteOptions;
    use rekordcrate::pdb::{RowGroup, Page};

    let mut reader = std::fs::File::open(inpath)?;
    let header = Header::read(&mut reader)?;

    println!("Header {:?}", header);

    let mut writer = std::fs::File::create(outpath)?;
    header.write(&mut writer)?;

    let write_options = &WriteOptions::new(binrw::Endian::NATIVE);

    for (_, table) in header.tables.iter().enumerate() {
        for page in header
            .read_pages(
                &mut reader,
                &ReadOptions::new(binrw::Endian::NATIVE),
                (&table.first_page, &table.last_page),
            )
            .unwrap()
            .into_iter()
        {
            println!("  {:?}", page);
            page.write_options(&mut writer, write_options, (header.page_size,))?;
            page.row_groups.iter().for_each(|row_group| {
                RowGroup::write(&row_group, &mut writer).unwrap();
                for row in row_group.present_rows() {
                    row.write(&mut writer).unwrap();
                }
            })
        }
    }

    Ok(())
}

fn dump_setting(path: &PathBuf) -> rekordcrate::Result<()> {
    let mut reader = std::fs::File::open(path)?;
    let setting = Setting::read(&mut reader)?;

    println!("{:#04x?}", setting);

    Ok(())
}

fn main() -> rekordcrate::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::ListPlaylists { path } => list_playlists(path),
        Commands::DumpPDB { path } => dump_pdb(path),
        Commands::ReexportPDB { inpath, outpath } => reexport_pdb(inpath, outpath),
        Commands::DumpANLZ { path } => dump_anlz(path),
        Commands::DumpSetting { path } => dump_setting(path),
    }
}
