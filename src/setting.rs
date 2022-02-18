// Copyright (c) 2022 Jan Holthuis
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy
// of the MPL was not distributed with this file, You can obtain one at
// http://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Parser for Rekordbox `*SETTING.DAT` files.
//!
//! These files either in the `PIONEER` directory of a USB drive (device exports), but are also
//! present for on local installations of Rekordbox 6 in `%APPDATA%\Pioneer\rekordbox6`.
//!
//! The settings files store the settings found on the "DJ System" > "My Settings" page of the
//! Rekordbox preferences. These includes language, LCD brightness, tempo fader range, crossfader
//! curve, etc.
//!
//! The exact format still has to be reverse-engineered.

use crate::util::nom_input_error_with_kind;
use nom::error::ErrorKind;
use nom::IResult;

#[derive(Debug)]
/// Represents a setting file.
pub struct Setting {
    /// Size of the string data field (should be always 96).
    pub len_stringdata: u32,
    /// Name of the company ("PIONEER").
    pub company: String,
    /// Name of the software ("rekordbox").
    pub software: String,
    /// Some kind of version number.
    pub version: String,
    /// Size of the `unknown1` data in bytes.
    pub len_unknown1: u32,
    /// Unknown field.
    pub unknown1: Vec<u8>,
    /// CRC16 XMODEM checksum. The checksum is calculated over the contents of the `unknown1`
    /// field, except for `DJMSETTING.DAT` files where the checksum is calculated over all
    /// preceding bytes including the length fields.
    ///
    /// See <https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-16-xmodem> for
    /// details.
    pub checksum: u16,
    /// Unknown field (apparently always `0000`).
    pub unknown2: u16,
}

impl Setting {
    /// Parses the Setting file and returns the structure.
    pub fn parse(orig_input: &[u8]) -> IResult<&[u8], Self> {
        let (input, len_stringdata) = nom::number::complete::le_u32(orig_input)?;
        let stringdata_size = usize::try_from(len_stringdata)
            .map_err(|_| nom_input_error_with_kind(input, ErrorKind::TooLarge))?;
        let stringdatasection_size = stringdata_size / 3;
        let (input, company) = nom::bytes::complete::take(stringdatasection_size)(input)?;
        let company = std::str::from_utf8(company)
            .unwrap()
            .trim_end_matches('\0')
            .to_owned();
        let (input, software) = nom::bytes::complete::take(stringdatasection_size)(input)?;
        let software = std::str::from_utf8(software)
            .unwrap()
            .trim_end_matches('\0')
            .to_owned();
        let (input, version) = nom::bytes::complete::take(stringdatasection_size)(input)?;
        let version = std::str::from_utf8(version)
            .unwrap()
            .trim_end_matches('\0')
            .to_owned();

        let (input, len_unknown1) = nom::number::complete::le_u32(input)?;
        let unknown1_size = usize::try_from(len_unknown1)
            .map_err(|_| nom_input_error_with_kind(input, ErrorKind::TooLarge))?;
        let (input, unknown1) = nom::bytes::complete::take(unknown1_size)(input)?;
        let unknown1 = unknown1.to_vec();
        let (input, checksum) = nom::number::complete::le_u16(input)?;
        let (input, unknown2) = nom::number::complete::le_u16(input)?;
        if !input.is_empty() {
            return Err(nom_input_error_with_kind(input, ErrorKind::Complete));
        }

        Ok((
            input,
            Self {
                len_stringdata,
                company,
                software,
                version,
                len_unknown1,
                unknown1,
                checksum,
                unknown2,
            },
        ))
    }
}
