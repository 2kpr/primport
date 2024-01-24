#![allow(dead_code)]
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

pub fn align(writer: &mut Cursor<Vec<u8>>, alignment: u8) {
    let pad = (alignment - (writer.position() % alignment as u64) as u8) % alignment;
    for _ in 0..pad {
        writer.write_u8(0).unwrap();
    }
}

pub fn read_f32_array(reader: &mut Cursor<Vec<u8>>, size: usize) -> Vec<f32> {
    let mut data = Vec::new();
    for _ in 0..size {
        data.push(reader.read_f32::<LittleEndian>().unwrap());
    }
    data
}

pub fn read_u8_array(reader: &mut Cursor<Vec<u8>>, size: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for _ in 0..size {
        data.push(reader.read_u8().unwrap());
    }
    data
}

pub fn read_u8_array_as_f32(reader: &mut Cursor<Vec<u8>>, size: usize) -> Vec<f32> {
    let mut data = Vec::new();
    for _ in 0..size {
        data.push(reader.read_u8().unwrap() as f32);
    }
    data
}

pub fn read_u16_array(reader: &mut Cursor<Vec<u8>>, size: usize) -> Vec<u16> {
    let mut data = Vec::new();
    for _ in 0..size {
        data.push(reader.read_u16::<LittleEndian>().unwrap());
    }
    data
}

pub fn read_u16_array_as_f32(reader: &mut Cursor<Vec<u8>>, size: usize) -> Vec<f32> {
    let mut data = Vec::new();
    for _ in 0..size {
        data.push(reader.read_u16::<LittleEndian>().unwrap() as f32);
    }
    data
}

pub fn read_u32_array(reader: &mut Cursor<Vec<u8>>, size: usize) -> Vec<u32> {
    let mut data = Vec::new();
    for _ in 0..size {
        data.push(reader.read_u32::<LittleEndian>().unwrap());
    }
    data
}

pub fn write_f32_into(writer: &mut Cursor<Vec<u8>>, data: &[f32]) {
    for i in 0..data.len() {
        writer.write_f32::<LittleEndian>(data[i]).unwrap();
    }
}

pub fn write_u8_into(writer: &mut Cursor<Vec<u8>>, data: &[u8]) {
    for i in 0..data.len() {
        writer.write_u8(data[i]).unwrap();
    }
}

pub fn write_u16_into(writer: &mut Cursor<Vec<u8>>, data: &[u16]) {
    for i in 0..data.len() {
        writer.write_u16::<LittleEndian>(data[i]).unwrap();
    }
}

pub fn write_u32_into(writer: &mut Cursor<Vec<u8>>, data: &[u32]) {
    for i in 0..data.len() {
        writer.write_u32::<LittleEndian>(data[i]).unwrap();
    }
}
