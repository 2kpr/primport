#![allow(dead_code)]
use super::io;
use super::GameVersion;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{Cursor, Seek, SeekFrom};
use std::io::{Read, Write};
use std::path::PathBuf;

struct Vertices {
    positions: Vec<[f32; 4]>,
    weights: Vec<([f32; 4], [f32; 2])>,
    bones: Vec<([u8; 4], [u8; 2])>,
    normals: Vec<[f32; 4]>,
    tangents: Vec<[f32; 4]>,
    bitangents: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[u8; 4]>,
}

impl Vertices {
    fn read(
        reader: &mut Cursor<Vec<u8>>,
        header_flags: u32,
        mesh: &SPrimMesh,
        sub_mesh: &SPrimSubMesh,
    ) -> Vertices {
        let has_alt_packing_format = mesh.object.header.draw_destination & 0x80 == 0x80;
        let has_hi_res_positions = mesh.object.flags & 8 == 8;
        let mut positions = Vec::new();
        if !has_alt_packing_format {
            for _ in 0..sub_mesh.num_vertices {
                Vertices::read_position(&mut positions, reader, mesh, has_hi_res_positions);
            }
        }
        let mut weights = Vec::new();
        let mut bones = Vec::new();
        let is_weighted = header_flags & 8 == 8;
        if is_weighted {
            Vertices::read_weights_and_bones(&mut weights, &mut bones, reader, sub_mesh);
        }
        let mut normals: Vec<[f32; 4]> = Vec::new();
        let mut tangents: Vec<[f32; 4]> = Vec::new();
        let mut bitangents: Vec<[f32; 4]> = Vec::new();
        let mut uvs = Vec::new();
        for _ in 0..sub_mesh.num_vertices {
            if has_alt_packing_format {
                Vertices::read_position(&mut positions, reader, mesh, has_hi_res_positions);
            }
            Vertices::read_vertex_data(&mut normals, reader);
            Vertices::read_vertex_data(&mut tangents, reader);
            Vertices::read_vertex_data(&mut bitangents, reader);
            Vertices::read_uv(&mut uvs, reader, mesh);
        }
        let mut colors = Vec::new();
        let has_color1_object = mesh.object.flags & 0x20 == 0x20;
        let has_color1_sub_mesh = sub_mesh.object.flags & 0x20 == 0x20;
        if is_weighted || !has_color1_object {
            if has_color1_sub_mesh {
                let color = {
                    if sub_mesh.object.color1.is_some() {
                        sub_mesh.object.color1.unwrap().to_le_bytes()
                    } else {
                        [0; 4]
                    }
                };
                for _ in 0..sub_mesh.num_vertices {
                    colors.push(color);
                }
            } else {
                for _ in 0..sub_mesh.num_vertices {
                    colors.push(io::read_u8_array(reader, 4).try_into().unwrap());
                }
            }
        }
        Vertices {
            positions: positions,
            weights: weights,
            bones: bones,
            normals: normals,
            tangents: tangents,
            bitangents: bitangents,
            uvs: uvs,
            colors: colors,
        }
    }

    fn read_position(
        positions: &mut Vec<[f32; 4]>,
        reader: &mut Cursor<Vec<u8>>,
        mesh: &SPrimMesh,
        has_hi_res_position: bool,
    ) {
        if has_hi_res_position {
            let mut position = io::read_f32_array(reader, 3);
            position.push(0.75);
            positions.push(position.try_into().unwrap());
        } else {
            let mut position: [f32; 4] = io::read_u16_array_as_f32(reader, 4).try_into().unwrap();
            position.iter_mut().enumerate().for_each(|(j, x)| {
                *x = (*x * mesh.position_scale[j]) / u16::MAX as f32 + mesh.position_bias[j]
            });
            positions.push(position);
        }
    }

    fn read_weights_and_bones(
        weights: &mut Vec<([f32; 4], [f32; 2])>,
        bones: &mut Vec<([u8; 4], [u8; 2])>,
        reader: &mut Cursor<Vec<u8>>,
        sub_mesh: &SPrimSubMesh,
    ) {
        for _ in 0..sub_mesh.num_vertices {
            let mut weight = ([0.0; 4], [0.0; 2]);
            let mut bone = ([0; 4], [0; 2]);
            weight.0 = io::read_u8_array_as_f32(reader, 4).try_into().unwrap();
            weight.0.iter_mut().for_each(|x| *x /= 255.0);
            bone.0 = io::read_u8_array(reader, 4).try_into().unwrap();
            weight.1 = io::read_u8_array_as_f32(reader, 2).try_into().unwrap();
            weight.1.iter_mut().for_each(|x| *x /= 255.0);
            bone.1 = io::read_u8_array(reader, 2).try_into().unwrap();
            weights.push(weight);
            bones.push(bone);
        }
    }

    fn read_vertex_data(values: &mut Vec<[f32; 4]>, reader: &mut Cursor<Vec<u8>>) {
        let mut value: [f32; 4] = io::read_u8_array_as_f32(reader, 4).try_into().unwrap();
        value
            .iter_mut()
            .for_each(|x| *x = ((2.0 * *x) / 255.0) - 1.0);
        values.push(value);
    }

    fn read_uv(uvs: &mut Vec<[f32; 2]>, reader: &mut Cursor<Vec<u8>>, mesh: &SPrimMesh) {
        let mut uv: [f32; 2] = io::read_u16_array_as_f32(reader, 2).try_into().unwrap();
        uv.iter_mut()
            .enumerate()
            .for_each(|(i, x)| *x = (*x * mesh.uv_scale[i]) / u16::MAX as f32 + mesh.uv_bias[i]);
        uvs.push(uv);
    }

    fn write(
        &self,
        writer: &mut Cursor<Vec<u8>>,
        header_flags: u32,
        mesh: &SPrimMesh,
        sub_mesh: &SPrimSubMesh,
    ) {
        let has_alt_packing_format = mesh.object.header.draw_destination & 0x80 == 0x80;
        let has_hi_res_positions = mesh.object.flags & 8 == 8;
        if !has_alt_packing_format {
            for i in 0..self.positions.len() {
                self.write_position(&self.positions[i], writer, mesh, has_hi_res_positions);
            }
        }
        let is_weighted = header_flags & 8 == 8;
        if is_weighted {
            self.write_weights_and_bones(writer);
        }
        for i in 0..self.positions.len() {
            if has_alt_packing_format {
                self.write_position(&self.positions[i], writer, mesh, has_hi_res_positions);
            }
            self.write_vertex_data(&self.normals[i], writer);
            self.write_vertex_data(&self.tangents[i], writer);
            self.write_vertex_data(&self.bitangents[i], writer);
            self.write_uv(&self.uvs[i], writer, mesh);
        }
        let has_color1_object = mesh.object.flags & 0x20 == 0x20;
        let has_color1_sub_mesh = sub_mesh.object.flags & 0x20 == 0x20;
        if is_weighted || !has_color1_object {
            if !has_color1_sub_mesh {
                for color in &self.colors {
                    io::write_u8_into(writer, color);
                }
            }
        }
        io::align(writer, 0x10);
    }

    fn write_position(
        &self,
        position: &[f32; 4],
        writer: &mut Cursor<Vec<u8>>,
        mesh: &SPrimMesh,
        has_hi_res_position: bool,
    ) {
        if has_hi_res_position {
            io::write_f32_into(writer, &position[0..3]);
        } else {
            let mut compressed: [u16; 4] = [0; 4];
            compressed.iter_mut().enumerate().for_each(|(i, x)| {
                *x = f32::round(
                    u16::MAX as f32 * (position[i] - mesh.position_bias[i])
                        / mesh.position_scale[i],
                ) as u16
            });
            io::write_u16_into(writer, &compressed);
        }
    }

    fn write_weights_and_bones(&self, writer: &mut Cursor<Vec<u8>>) {
        for i in 0..self.weights.len() {
            let mut weights: [u8; 4] = [0; 4];
            weights
                .iter_mut()
                .enumerate()
                .for_each(|(j, x)| *x = f32::round(self.weights[i].0[j] * 255.0) as u8);
            io::write_u8_into(writer, &weights);
            io::write_u8_into(writer, &self.bones[i].0);
            let mut weights: [u8; 2] = [0; 2];
            weights
                .iter_mut()
                .enumerate()
                .for_each(|(j, x)| *x = f32::round(self.weights[i].1[j] * 255.0) as u8);
            io::write_u8_into(writer, &weights);
            io::write_u8_into(writer, &self.bones[i].1);
        }
    }

    fn write_vertex_data(&self, value: &[f32; 4], writer: &mut Cursor<Vec<u8>>) {
        let mut compressed: [u8; 4] = [0; 4];
        compressed
            .iter_mut()
            .enumerate()
            .for_each(|(j, x)| *x = f32::round(((value[j] + 1.0) / 2.0) * 255.0) as u8);
        io::write_u8_into(writer, &compressed);
    }

    fn write_uv(&self, uv: &[f32; 2], writer: &mut Cursor<Vec<u8>>, mesh: &SPrimMesh) {
        let mut compressed: [u16; 2] = [0; 2];
        compressed.iter_mut().enumerate().for_each(|(j, x)| {
            *x = f32::round(u16::MAX as f32 * (uv[j] - mesh.uv_bias[j]) / mesh.uv_scale[j]) as u16
        });
        io::write_u16_into(writer, &compressed);
    }

    fn print(&self) {
        println!("vertices: {:#x?}", self.positions);
        println!("weights: {:#x?}", self.weights);
        println!("bones: {:#x?}", self.bones);
        println!("normals: {:#x?}", self.normals);
        println!("tangents: {:#x?}", self.tangents);
        println!("bitangents: {:#x?}", self.bitangents);
        println!("uvs: {:#x?}", self.uvs);
        println!("colors: {:#x?}", self.colors);
    }
}

struct Indices {
    indices: Vec<u16>,
}

impl Indices {
    fn read(reader: &mut Cursor<Vec<u8>>, sub_mesh: &SPrimSubMesh) -> Indices {
        let count = sub_mesh.num_indices + {
            if sub_mesh.num_indices_extra.is_some() {
                *sub_mesh.num_indices_extra.as_ref().unwrap()
            } else {
                0
            }
        };
        let indices = io::read_u16_array(reader, count as usize);
        Indices { indices: indices }
    }

    fn write(&self, writer: &mut Cursor<Vec<u8>>) {
        io::write_u16_into(writer, &self.indices);
        io::align(writer, 0x10);
    }

    fn print(&self) {
        println!("indices: {:#x?}", self.indices);
    }
}

struct Collision {
    bounding_boxes: Vec<[[u8; 3]; 2]>,
    triangles_per_box: u16,
}

impl Collision {
    fn read(reader: &mut Cursor<Vec<u8>>) -> Collision {
        let count = reader.read_u16::<LittleEndian>().unwrap();
        let triangles_per_box = reader.read_u16::<LittleEndian>().unwrap();
        let mut bounding_boxes = Vec::new();
        for _ in 0..count {
            bounding_boxes.push([
                io::read_u8_array(reader, 3).try_into().unwrap(),
                io::read_u8_array(reader, 3).try_into().unwrap(),
            ]);
        }
        Collision {
            bounding_boxes: bounding_boxes,
            triangles_per_box: triangles_per_box,
        }
    }

    fn write(&self, writer: &mut Cursor<Vec<u8>>) {
        writer
            .write_u16::<LittleEndian>(self.bounding_boxes.len() as u16)
            .unwrap();
        writer
            .write_u16::<LittleEndian>(self.triangles_per_box)
            .unwrap();
        for bounding_box in &self.bounding_boxes {
            io::write_u8_into(writer, &bounding_box[0]);
            io::write_u8_into(writer, &bounding_box[1]);
        }
        io::align(writer, 0x10);
    }

    fn print(&self) {
        println!("bounding_boxes: {:#x?}", self.bounding_boxes);
        println!("triangles_per_box: {:#x}", self.triangles_per_box);
    }
}

struct Cloth {
    data: Vec<u8>,
}

impl Cloth {
    fn read(reader: &mut Cursor<Vec<u8>>, sub_mesh: &SPrimSubMesh, cloth_id: u8) -> Cloth {
        let is_small = cloth_id & 0x80 == 0x80;
        let mut data = Vec::new();
        let size = {
            if is_small {
                let size = reader.read_u32::<LittleEndian>().unwrap();
                data.extend_from_slice(size.to_le_bytes().as_slice());
                size
            } else {
                sub_mesh.num_vertices * 0x14
            }
        };
        data.extend_from_slice(&io::read_u8_array(reader, size as usize).as_slice());
        Cloth { data: data }
    }

    fn write(&self, writer: &mut Cursor<Vec<u8>>) {
        io::write_u8_into(writer, &self.data);
        io::align(writer, 0x10);
    }

    fn print(&self) {
        println!("data: {:#x?}", self.data);
    }
}

struct CopyBones {
    data: Vec<u32>,
}

impl CopyBones {
    fn read(reader: &mut Cursor<Vec<u8>>, num_copy_bones: u32) -> CopyBones {
        let size = num_copy_bones * 2;
        let data = io::read_u32_array(reader, size as usize);
        CopyBones { data: data }
    }

    fn write(&self, writer: &mut Cursor<Vec<u8>>) {
        io::write_u32_into(writer, &self.data);
        io::align(writer, 0x10);
    }

    fn print(&self) {
        println!("data: {:#x?}", self.data);
    }
}

struct BoneIndices {
    data: Vec<u16>,
}

impl BoneIndices {
    fn read(reader: &mut Cursor<Vec<u8>>, input_version: &GameVersion) -> BoneIndices {
        let size = match input_version {
            GameVersion::Hma | GameVersion::Alpha => {
                reader.read_u16::<LittleEndian>().unwrap() as u32 - 1
            }
            GameVersion::Hm2016 | GameVersion::Woa => {
                reader.read_u32::<LittleEndian>().unwrap() - 2
            }
        };
        let data = io::read_u16_array(reader, size as usize);
        BoneIndices { data: data }
    }

    fn write(&self, writer: &mut Cursor<Vec<u8>>, output_version: &GameVersion) {
        match output_version {
            GameVersion::Hma | GameVersion::Alpha => writer
                .write_u16::<LittleEndian>((self.data.len() + 1) as u16)
                .unwrap(),
            GameVersion::Hm2016 | GameVersion::Woa => writer
                .write_u32::<LittleEndian>((self.data.len() + 2) as u32)
                .unwrap(),
        };
        io::write_u16_into(writer, &self.data);
        io::align(writer, 0x10);
    }

    fn print(&self) {
        println!("data: {:#x?}", self.data);
    }
}

struct BoneInfo {
    data: Vec<u8>,
}

impl BoneInfo {
    fn read(reader: &mut Cursor<Vec<u8>>) -> BoneInfo {
        let size = reader.read_u16::<LittleEndian>().unwrap();
        reader.seek(SeekFrom::Current(-2)).unwrap();
        let data = io::read_u8_array(reader, size as usize);
        BoneInfo { data: data }
    }

    fn write(&self, writer: &mut Cursor<Vec<u8>>) {
        io::write_u8_into(writer, &self.data);
        io::align(writer, 0x10);
    }

    fn print(&self) {
        println!("data: {:#x?}", self.data);
    }
}

#[repr(i16)]
enum EPrimType {
    None = 0,
    ObjectHeader = 1,
    Mesh = 2,
    Shape = 5,
}

struct SPrimHeader {
    draw_destination: u8,
    pack_type: u8,
    prim_type: u16,
}

impl SPrimHeader {
    fn read(reader: &mut Cursor<Vec<u8>>) -> SPrimHeader {
        SPrimHeader {
            draw_destination: reader.read_u8().unwrap(),
            pack_type: reader.read_u8().unwrap(),
            prim_type: reader.read_u16::<LittleEndian>().unwrap(),
        }
    }

    fn write(&self, writer: &mut Cursor<Vec<u8>>) {
        writer.write_u8(self.draw_destination).unwrap();
        writer.write_u8(self.pack_type).unwrap();
        writer.write_u16::<LittleEndian>(self.prim_type).unwrap();
    }

    fn print(&self) {
        println!("draw_destination: {:#x}", self.draw_destination);
        println!("pack_type: {:#x}", self.pack_type);
        println!("prim_type: {:#x}", self.prim_type);
    }
}

#[repr(i32)]
enum ObjectHeaderFlags {
    HasBones = 1,
    HasFrames = 2,
    IsLinkedObject = 4,
    IsWeightedObject = 8,
    UseBounds = 0x100,
    HasHiResPositions = 0x200,
}

enum Object {
    SPrimMesh(SPrimMesh),
    SPrimMeshWeighted(SPrimMeshWeighted),
}

struct SPrimObjectHeader {
    header: SPrimHeader,
    header_flags: u32,
    bone_rig_resource_index: u32,
    num_objects: u32,
    object_table: u32,
    bounding_box_min: [f32; 3],
    bounding_box_max: [f32; 3],
    objects: Vec<Object>,
}

impl SPrimObjectHeader {
    fn read(reader: &mut Cursor<Vec<u8>>) -> SPrimObjectHeader {
        SPrimObjectHeader {
            header: SPrimHeader::read(reader),
            header_flags: reader.read_u32::<LittleEndian>().unwrap(),
            bone_rig_resource_index: reader.read_u32::<LittleEndian>().unwrap(),
            num_objects: reader.read_u32::<LittleEndian>().unwrap(),
            object_table: reader.read_u32::<LittleEndian>().unwrap(),
            bounding_box_min: [
                reader.read_f32::<LittleEndian>().unwrap(),
                reader.read_f32::<LittleEndian>().unwrap(),
                reader.read_f32::<LittleEndian>().unwrap(),
            ],
            bounding_box_max: [
                reader.read_f32::<LittleEndian>().unwrap(),
                reader.read_f32::<LittleEndian>().unwrap(),
                reader.read_f32::<LittleEndian>().unwrap(),
            ],
            objects: Vec::new(),
        }
    }

    fn read_objects(
        &mut self,
        reader: &mut Cursor<Vec<u8>>,
        input_version: &GameVersion,
        verbose: bool,
    ) {
        for o in 0..self.num_objects {
            reader
                .seek(SeekFrom::Start((self.object_table + o * 4) as u64))
                .unwrap();
            let object_offset = reader.read_u32::<LittleEndian>().unwrap();
            reader.seek(SeekFrom::Start(object_offset as u64)).unwrap();
            if self.is_weighted() {
                let object = SPrimMeshWeighted::read(reader, self.header_flags, &input_version);
                if verbose {
                    object.print();
                }
                self.objects.push(Object::SPrimMeshWeighted(object));
            } else {
                let object: SPrimMesh = SPrimMesh::read(reader, self.header_flags, &input_version);
                if verbose {
                    object.print();
                }
                self.objects.push(Object::SPrimMesh(object));
            }
        }
    }

    fn write(
        &mut self,
        writer: &mut Cursor<Vec<u8>>,
        output_version: &GameVersion,
        no_cloth: &bool,
    ) -> u32 {
        let mut object_offsets = Vec::new();
        for object in &mut self.objects {
            match object {
                Object::SPrimMesh(object) => {
                    object.object.header.draw_destination = SPrimObjectHeader::get_draw_destination(
                        self.header_flags,
                        object.object.header.draw_destination,
                        output_version,
                    );
                    object_offsets.push(object.write(
                        writer,
                        self.header_flags,
                        &output_version,
                        0xFFFFFFFF,
                    ));
                    io::align(writer, 0x10);
                }
                Object::SPrimMeshWeighted(object) => {
                    if !no_cloth
                        || (object.mesh.sub_mesh.as_ref().unwrap().offset_cloth > 0
                            || (object.mesh.sub_mesh.as_ref().unwrap().offset_cloth == 0
                                && object.mesh.cloth_id == 0))
                    {
                        object.mesh.object.header.draw_destination =
                            SPrimObjectHeader::get_draw_destination(
                                self.header_flags,
                                object.mesh.object.header.draw_destination,
                                output_version,
                            );
                        object_offsets.push(object.write(
                            writer,
                            self.header_flags,
                            &output_version,
                        ));
                    }
                }
            }
        }
        let object_table = writer.position() as u32;
        io::write_u32_into(writer, &object_offsets);
        io::align(writer, 0x10);
        let main_offset = writer.position() as u32;
        self.header.write(writer);
        writer.write_u32::<LittleEndian>(self.header_flags).unwrap();
        writer
            .write_u32::<LittleEndian>(self.bone_rig_resource_index)
            .unwrap();
        writer
            .write_u32::<LittleEndian>(object_offsets.len() as u32)
            .unwrap();
        writer.write_u32::<LittleEndian>(object_table).unwrap();
        io::write_f32_into(writer, &self.bounding_box_min);
        io::write_f32_into(writer, &self.bounding_box_max);
        io::align(writer, 0x10);
        main_offset
    }

    fn get_draw_destination(
        header_flags: u32,
        draw_destination: u8,
        output_version: &GameVersion,
    ) -> u8 {
        match output_version {
            GameVersion::Alpha => {
                if header_flags & 8 == 8 {
                    draw_destination & 0xF
                } else {
                    0x81
                }
            }
            _ => draw_destination & 0xF,
        }
    }

    fn print(&self) {
        self.header.print();
        println!("header_flags: {:#x}", self.header_flags);
        println!(
            "bone_rig_resource_index: {:#x}",
            self.bone_rig_resource_index
        );
        println!("num_objects: {:#x}", self.num_objects);
        println!("object_table: {:#x}", self.object_table);
        println!("bounding_box_min: {:#x?}", self.bounding_box_min);
        println!("bounding_box_max: {:#x?}", self.bounding_box_max);
    }

    fn is_weighted(&self) -> bool {
        return self.header_flags & 8 == 8;
    }
}

#[repr(u8)]
enum SubType {
    Standard = 0,
    Linked = 1,
    Weighted = 2,
    StandardUv2 = 3,
    StandardUv3 = 4,
    StandardUv4 = 5,
    SpeedTree = 6,
}

#[repr(u8)]
enum ObjectFlags {
    None = 0,
    XAxisLocked = 1,
    YAxisLocked = 2,
    ZAxisLocked = 4,
    HiResPositions = 8,
    Ps3Edge = 0x10,
    Color1 = 0x20,
    IsNoPhysicsProp = 0x40,
}

struct SPrimObject {
    header: SPrimHeader,
    sub_type: u8,
    flags: u8,
    lod_mask: u8,
    variant_id: u8,
    bias: u8,
    offset: u8,
    material_id: u16,
    wire_color: u32,
    color1: Option<u32>,
    bounding_box_min: [f32; 3],
    bounding_box_max: [f32; 3],
}

impl SPrimObject {
    fn read(reader: &mut Cursor<Vec<u8>>, input_version: &GameVersion) -> SPrimObject {
        SPrimObject {
            header: SPrimHeader::read(reader),
            sub_type: reader.read_u8().unwrap(),
            flags: reader.read_u8().unwrap(),
            lod_mask: reader.read_u8().unwrap(),
            variant_id: reader.read_u8().unwrap(),
            bias: reader.read_u8().unwrap(),
            offset: reader.read_u8().unwrap(),
            material_id: reader.read_u16::<LittleEndian>().unwrap(),
            wire_color: reader.read_u32::<LittleEndian>().unwrap(),
            color1: {
                match input_version {
                    GameVersion::Hma | GameVersion::Alpha => None,
                    GameVersion::Hm2016 | GameVersion::Woa => {
                        Some(reader.read_u32::<LittleEndian>().unwrap())
                    }
                }
            },
            bounding_box_min: io::read_f32_array(reader, 3).try_into().unwrap(),
            bounding_box_max: io::read_f32_array(reader, 3).try_into().unwrap(),
        }
    }

    fn write(&self, writer: &mut Cursor<Vec<u8>>, output_version: &GameVersion) {
        self.header.write(writer);
        writer.write_u8(self.sub_type).unwrap();
        writer.write_u8(self.flags).unwrap();
        writer.write_u8(self.lod_mask).unwrap();
        writer.write_u8(self.variant_id).unwrap();
        writer.write_u8(self.bias).unwrap();
        writer.write_u8(self.offset).unwrap();
        writer.write_u16::<LittleEndian>(self.material_id).unwrap();
        writer.write_u32::<LittleEndian>(self.wire_color).unwrap();
        let color1 = {
            if self.color1.is_some() {
                *self.color1.as_ref().unwrap()
            } else {
                0
            }
        };
        match output_version {
            GameVersion::Hma | GameVersion::Alpha => (),
            GameVersion::Hm2016 | GameVersion::Woa => {
                writer.write_u32::<LittleEndian>(color1).unwrap()
            }
        }
        io::write_f32_into(writer, &self.bounding_box_min);
        io::write_f32_into(writer, &self.bounding_box_max);
    }

    fn print(&self) {
        self.header.print();
        println!("sub_type: {:#x}", self.sub_type);
        println!("flags: {:#x}", self.flags);
        println!("lod_mask: {:#x}", self.lod_mask);
        println!("variant_id: {:#x}", self.variant_id);
        println!("bias: {:#x}", self.bias);
        println!("offset: {:#x}", self.offset);
        println!("material_id: {:#x}", self.material_id);
        println!("wire_color: {:#x}", self.wire_color);
        if self.color1.is_some() {
            println!("color1: {:#x}", self.color1.as_ref().unwrap());
        }
        println!("bounding_box_min: {:#x?}", self.bounding_box_min);
        println!("bounding_box_max: {:#x?}", self.bounding_box_max);
    }
}

#[repr(i32)]
enum ClothFlags {
    Small = 0x80,
}

struct SPrimMesh {
    object: SPrimObject,
    sub_mesh_table: u32,
    sub_mesh: Option<SPrimSubMesh>,
    position_scale: [f32; 4],
    position_bias: [f32; 4],
    uv_scale: [f32; 2],
    uv_bias: [f32; 2],
    cloth_id: u8,
    pad: [u8; 3],
}

impl SPrimMesh {
    fn read(
        reader: &mut Cursor<Vec<u8>>,
        header_flags: u32,
        input_version: &GameVersion,
    ) -> SPrimMesh {
        let mut mesh = SPrimMesh {
            object: SPrimObject::read(reader, &input_version),
            sub_mesh_table: reader.read_u32::<LittleEndian>().unwrap(),
            sub_mesh: None,
            position_scale: io::read_f32_array(reader, 4).try_into().unwrap(),
            position_bias: io::read_f32_array(reader, 4).try_into().unwrap(),
            uv_scale: io::read_f32_array(reader, 2).try_into().unwrap(),
            uv_bias: io::read_f32_array(reader, 2).try_into().unwrap(),
            cloth_id: reader.read_u8().unwrap(),
            pad: io::read_u8_array(reader, 3).try_into().unwrap(),
        };
        let position = reader.position();
        reader
            .seek(SeekFrom::Start(mesh.sub_mesh_table as u64))
            .unwrap();
        let object_offset = reader.read_u32::<LittleEndian>().unwrap();
        reader.seek(SeekFrom::Start(object_offset as u64)).unwrap();
        mesh.sub_mesh = Some(SPrimSubMesh::read(
            reader,
            header_flags,
            &mesh,
            &input_version,
        ));
        reader.seek(SeekFrom::Start(position)).unwrap();
        mesh
    }

    fn write_sub_mesh(
        &self,
        writer: &mut Cursor<Vec<u8>>,
        header_flags: u32,
        output_version: &GameVersion,
    ) -> u32 {
        if self.sub_mesh.is_some() {
            self.sub_mesh
                .as_ref()
                .unwrap()
                .write(writer, header_flags, &self, output_version)
        } else {
            0
        }
    }

    fn write(
        &self,
        writer: &mut Cursor<Vec<u8>>,
        header_flags: u32,
        output_version: &GameVersion,
        sub_mesh_table: u32,
    ) -> u32 {
        let offset_sub_mesh_table = {
            if sub_mesh_table == 0xFFFFFFFF {
                self.sub_mesh
                    .as_ref()
                    .unwrap()
                    .write(writer, header_flags, &self, output_version)
            } else {
                sub_mesh_table
            }
        };
        let offset = writer.position() as u32;
        self.object.write(writer, output_version);
        writer
            .write_u32::<LittleEndian>(offset_sub_mesh_table)
            .unwrap();
        //io::align(writer, 0x10);
        io::write_f32_into(writer, &self.position_scale);
        io::write_f32_into(writer, &self.position_bias);
        io::write_f32_into(writer, &self.uv_scale);
        io::write_f32_into(writer, &self.uv_bias);
        writer.write_u8(self.cloth_id).unwrap();
        io::write_u8_into(writer, &self.pad);
        offset
    }

    fn print(&self) {
        self.object.print();
        println!("sub_mesh_table: {:#x}", self.sub_mesh_table);
        self.sub_mesh.as_ref().unwrap().print();
        println!("position_scale: {:#x?}", self.position_scale);
        println!("position_bias: {:#x?}", self.position_bias);
        println!("uv_scale: {:#x?}", self.uv_scale);
        println!("uv_bias: {:#x?}", self.uv_bias);
        println!("cloth_id: {:#x}", self.cloth_id);
        println!("pad: {:#x?}", self.pad);
    }
}

struct SPrimMeshWeighted {
    mesh: SPrimMesh,
    num_copy_bones: u32,
    offset_copy_bones: u32,
    copy_bones: Option<CopyBones>,
    offset_bone_indicies: u32,
    bone_indicies: Option<BoneIndices>,
    offset_bone_info: u32,
    bone_info: Option<BoneInfo>,
}

impl SPrimMeshWeighted {
    fn read(
        reader: &mut Cursor<Vec<u8>>,
        header_flags: u32,
        input_version: &GameVersion,
    ) -> SPrimMeshWeighted {
        let mut mesh = SPrimMeshWeighted {
            mesh: SPrimMesh::read(reader, header_flags, &input_version),
            num_copy_bones: reader.read_u32::<LittleEndian>().unwrap(),
            offset_copy_bones: reader.read_u32::<LittleEndian>().unwrap(),
            copy_bones: None,
            offset_bone_indicies: reader.read_u32::<LittleEndian>().unwrap(),
            bone_indicies: None,
            offset_bone_info: reader.read_u32::<LittleEndian>().unwrap(),
            bone_info: None,
        };
        if mesh.num_copy_bones > 0 && mesh.offset_copy_bones > 0 {
            reader
                .seek(SeekFrom::Start(mesh.offset_copy_bones as u64))
                .unwrap();
            mesh.copy_bones = Some(CopyBones::read(reader, mesh.num_copy_bones));
        }
        if mesh.offset_bone_indicies > 0 {
            reader
                .seek(SeekFrom::Start(mesh.offset_bone_indicies as u64))
                .unwrap();
            mesh.bone_indicies = Some(BoneIndices::read(reader, &input_version));
        }
        if mesh.offset_bone_info > 0 {
            reader
                .seek(SeekFrom::Start(mesh.offset_bone_info as u64))
                .unwrap();
            mesh.bone_info = Some(BoneInfo::read(reader));
        }
        mesh
    }

    fn write(
        &self,
        writer: &mut Cursor<Vec<u8>>,
        header_flags: u32,
        output_version: &GameVersion,
    ) -> u32 {
        let sub_mesh_table = self
            .mesh
            .write_sub_mesh(writer, header_flags, &output_version);
        let mut offset_copy_bones = 0;
        if self.copy_bones.is_some() {
            offset_copy_bones = writer.position() as u32;
            self.copy_bones.as_ref().unwrap().write(writer);
        }
        io::align(writer, 0x10);
        let mut offset_bone_info = 0;
        if self.bone_info.is_some() {
            offset_bone_info = writer.position() as u32;
            self.bone_info.as_ref().unwrap().write(writer);
        }
        io::align(writer, 0x10);
        let mut offset_bone_indicies = 0;
        if self.bone_indicies.is_some() {
            offset_bone_indicies = writer.position() as u32;
            self.bone_indicies
                .as_ref()
                .unwrap()
                .write(writer, &output_version);
        }
        io::align(writer, 0x10);
        let offset = writer.position() as u32;
        self.mesh
            .write(writer, header_flags, &output_version, sub_mesh_table);
        writer
            .write_u32::<LittleEndian>(self.num_copy_bones)
            .unwrap();
        writer.write_u32::<LittleEndian>(offset_copy_bones).unwrap();
        writer
            .write_u32::<LittleEndian>(offset_bone_indicies)
            .unwrap();
        writer.write_u32::<LittleEndian>(offset_bone_info).unwrap();
        io::align(writer, 0x10);
        offset
    }

    fn print(&self) {
        self.mesh.print();
        println!("num_copy_bones: {:#x}", self.num_copy_bones);
        println!("offset_copy_bones: {:#x}", self.offset_copy_bones);
        if self.copy_bones.is_some() {
            self.copy_bones.as_ref().unwrap().print();
        }
        println!("offset_bone_indicies: {:#x}", self.offset_bone_indicies);
        if self.bone_indicies.is_some() {
            self.bone_indicies.as_ref().unwrap().print();
        }
        println!("offset_bone_info: {:#x}", self.offset_bone_info);
        if self.bone_info.is_some() {
            self.bone_info.as_ref().unwrap().print();
        }
    }
}

struct SPrimSubMesh {
    object: SPrimObject,
    num_vertices: u32,
    offset_vertices: u32,
    vertices: Option<Vertices>,
    num_indices: u32,
    num_indices_extra: Option<u32>,
    offset_indices: u32,
    indices: Option<Indices>,
    offset_collision: u32,
    collision: Option<Collision>,
    offset_cloth: u32,
    cloth: Option<Cloth>,
    num_uv_channels: u32,
}

impl SPrimSubMesh {
    fn read(
        reader: &mut Cursor<Vec<u8>>,
        header_flags: u32,
        mesh: &SPrimMesh,
        input_version: &GameVersion,
    ) -> SPrimSubMesh {
        let mut sub_mesh = SPrimSubMesh {
            object: SPrimObject::read(reader, &input_version),
            num_vertices: reader.read_u32::<LittleEndian>().unwrap(),
            offset_vertices: reader.read_u32::<LittleEndian>().unwrap(),
            vertices: None,
            num_indices: reader.read_u32::<LittleEndian>().unwrap(),
            num_indices_extra: {
                match input_version {
                    GameVersion::Hma | GameVersion::Alpha => None,
                    GameVersion::Hm2016 | GameVersion::Woa => {
                        Some(reader.read_u32::<LittleEndian>().unwrap())
                    }
                }
            },
            offset_indices: reader.read_u32::<LittleEndian>().unwrap(),
            indices: None,
            offset_collision: reader.read_u32::<LittleEndian>().unwrap(),
            collision: None,
            offset_cloth: reader.read_u32::<LittleEndian>().unwrap(),
            cloth: None,
            num_uv_channels: reader.read_u32::<LittleEndian>().unwrap(),
        };
        if sub_mesh.num_vertices > 0 && sub_mesh.offset_vertices > 0 {
            reader
                .seek(SeekFrom::Start(sub_mesh.offset_vertices as u64))
                .unwrap();
            sub_mesh.vertices = Some(Vertices::read(reader, header_flags, mesh, &sub_mesh));
        }
        if sub_mesh.num_indices > 0 && sub_mesh.offset_indices > 0 {
            reader
                .seek(SeekFrom::Start(sub_mesh.offset_indices as u64))
                .unwrap();
            sub_mesh.indices = Some(Indices::read(reader, &sub_mesh));
        }
        if sub_mesh.offset_collision > 0 {
            reader
                .seek(SeekFrom::Start(sub_mesh.offset_collision as u64))
                .unwrap();
            sub_mesh.collision = Some(Collision::read(reader));
        }
        if sub_mesh.offset_cloth > 0 {
            match input_version {
                GameVersion::Hm2016 | GameVersion::Woa => {
                    reader
                        .seek(SeekFrom::Start(sub_mesh.offset_cloth as u64))
                        .unwrap();
                    sub_mesh.cloth = Some(Cloth::read(reader, &sub_mesh, mesh.cloth_id));
                }
                _ => (),
            }
        }
        sub_mesh
    }

    fn write(
        &self,
        writer: &mut Cursor<Vec<u8>>,
        header_flags: u32,
        mesh: &SPrimMesh,
        output_version: &GameVersion,
    ) -> u32 {
        let mut offset_indices = 0;
        if self.indices.is_some() {
            offset_indices = writer.position() as u32;
            self.indices.as_ref().unwrap().write(writer);
        }
        io::align(writer, 0x10);
        let mut offset_vertices = 0;
        if self.vertices.is_some() {
            offset_vertices = writer.position() as u32;
            self.vertices
                .as_ref()
                .unwrap()
                .write(writer, header_flags, mesh, &self);
        }
        io::align(writer, 0x10);
        let mut offset_collision = 0;
        if self.collision.is_some() {
            offset_collision = writer.position() as u32;
            self.collision.as_ref().unwrap().write(writer);
        }
        io::align(writer, 0x10);
        let mut offset_cloth = 0;
        if self.cloth.is_some() {
            offset_cloth = writer.position() as u32;
            self.cloth.as_ref().unwrap().write(writer);
        }
        io::align(writer, 0x10);
        let offset_object = writer.position() as u32;
        self.object.write(writer, output_version);
        writer.write_u32::<LittleEndian>(self.num_vertices).unwrap();
        writer.write_u32::<LittleEndian>(offset_vertices).unwrap();
        writer.write_u32::<LittleEndian>(self.num_indices).unwrap();
        let num_indices_extra = {
            if self.num_indices_extra.is_some() {
                *self.num_indices_extra.as_ref().unwrap()
            } else {
                0
            }
        };
        match output_version {
            GameVersion::Hma | GameVersion::Alpha => (),
            GameVersion::Hm2016 | GameVersion::Woa => {
                writer.write_u32::<LittleEndian>(num_indices_extra).unwrap()
            }
        }
        writer.write_u32::<LittleEndian>(offset_indices).unwrap();
        writer.write_u32::<LittleEndian>(offset_collision).unwrap();
        writer.write_u32::<LittleEndian>(offset_cloth).unwrap();
        let is_weighted = header_flags & 8 == 8;
        let num_uv_channels = match output_version {
            GameVersion::Hma | GameVersion::Alpha | GameVersion::Hm2016 => {
                if is_weighted {
                    0
                } else {
                    1
                }
            }
            GameVersion::Woa => 1,
        };
        writer.write_u32::<LittleEndian>(num_uv_channels).unwrap();
        io::align(writer, 0x10);
        let offset = writer.position() as u32;
        writer.write_u32::<LittleEndian>(offset_object).unwrap();
        io::align(writer, 0x10);
        offset
    }

    fn print(&self) {
        self.object.print();
        println!("num_vertices: {:#x}", self.num_vertices);
        println!("offset_vertices: {:#x}", self.offset_vertices);
        if self.vertices.is_some() {
            self.vertices.as_ref().unwrap().print();
        }
        println!("num_indices: {:#x}", self.num_indices);
        if self.num_indices_extra.is_some() {
            println!(
                "num_indices_extra: {:#x}",
                self.num_indices_extra.as_ref().unwrap()
            );
        }
        println!("offset_indices: {:#x}", self.offset_indices);
        if self.indices.is_some() {
            self.indices.as_ref().unwrap().print();
        }
        println!("offset_collision: {:#x}", self.offset_collision);
        if self.collision.is_some() {
            self.collision.as_ref().unwrap().print();
        }
        println!("offset_cloth: {:#x}", self.offset_cloth);
        if self.cloth.is_some() {
            self.cloth.as_ref().unwrap().print();
        }
        println!("num_uv_channels: {:#x}", self.num_uv_channels);
    }
}

pub struct Prim {
    header: SPrimObjectHeader,
}

impl Prim {
    pub fn read(path: &PathBuf, input_version: &GameVersion, verbose: bool) -> Prim {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(err) => {
                println!("Error opening file {}: {}", path.to_str().unwrap(), err);
                std::process::exit(1);
            }
        };
        let mut buffer = vec![0 as u8; file.metadata().unwrap().len() as usize];
        Read::read(&mut file, &mut buffer).unwrap();
        let mut reader = Cursor::new(buffer);
        let main_offset = reader.read_u32::<LittleEndian>().unwrap();
        reader.seek(SeekFrom::Start(main_offset as u64)).unwrap();
        let mut header = SPrimObjectHeader::read(&mut reader);
        if verbose {
            println!("Prim Main Header: {:#x}", main_offset);
            header.print();
        }
        header.read_objects(&mut reader, &input_version, verbose);
        Prim { header: header }
    }

    pub fn write(&mut self, path: &PathBuf, output_version: &GameVersion, no_cloth: bool) {
        let mut file = match File::create(path) {
            Ok(file) => file,
            Err(err) => {
                println!("Error creating file {}: {}", path.to_str().unwrap(), err);
                std::process::exit(1);
            }
        };
        let buffer: Vec<u8> = Vec::new();
        let mut writer = Cursor::new(buffer);
        writer.write_u128::<LittleEndian>(0).unwrap();
        let main_offset = self.header.write(&mut writer, &output_version, &no_cloth);
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.write_u32::<LittleEndian>(main_offset).unwrap();
        writer.seek(SeekFrom::Start(0)).unwrap();
        File::write(&mut file, writer.get_ref()).unwrap();
    }
}
