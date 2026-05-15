use byteorder::{LittleEndian, ReadBytesExt};
use serde::Serialize;
use std::io::{Cursor, Read};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("packet too short: {0} bytes (need ≥311)")]
    TooShort(usize),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryPacket {
    pub is_race_on: bool,
    pub timestamp_ms: u32,
    pub engine_max_rpm: f32,
    pub engine_idle_rpm: f32,
    pub current_engine_rpm: f32,
    pub accel_x: f32,
    pub accel_y: f32,
    pub accel_z: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub vel_z: f32,
    pub tire_slip_ratio_fl: f32,
    pub tire_slip_ratio_fr: f32,
    pub tire_slip_ratio_rl: f32,
    pub tire_slip_ratio_rr: f32,
    pub tire_slip_angle_fl: f32,
    pub tire_slip_angle_fr: f32,
    pub tire_slip_angle_rl: f32,
    pub tire_slip_angle_rr: f32,
    pub car_ordinal: i32,
    pub car_class: i32,
    pub car_pi: i32,
    pub drivetrain_type: i32,
    pub speed_ms: f32,
    pub power: f32,
    pub torque: f32,
    pub tire_temp_fl: f32,
    pub tire_temp_fr: f32,
    pub tire_temp_rl: f32,
    pub tire_temp_rr: f32,
    pub boost: f32,
    pub fuel: f32,
    pub distance_traveled: f32,
    pub best_lap: f32,
    pub last_lap: f32,
    pub current_lap: f32,
    pub current_race_time: f32,
    pub lap_number: u16,
    pub race_position: u8,
    pub throttle: u8,
    pub brake: u8,
    pub clutch: u8,
    pub handbrake: u8,
    pub gear: u8,
    pub tire_wear_fl: Option<f32>,
    pub tire_wear_fr: Option<f32>,
    pub tire_wear_rl: Option<f32>,
    pub tire_wear_rr: Option<f32>,
}

pub fn parse(buf: &[u8]) -> Result<TelemetryPacket, ParseError> {
    if buf.len() < 311 {
        return Err(ParseError::TooShort(buf.len()));
    }
    let mut c = Cursor::new(buf);

    // Sled fields (bytes 0–231)
    let is_race_on = c.read_i32::<LittleEndian>()? != 0;
    let timestamp_ms = c.read_u32::<LittleEndian>()?;
    let engine_max_rpm = c.read_f32::<LittleEndian>()?;
    let engine_idle_rpm = c.read_f32::<LittleEndian>()?;
    let current_engine_rpm = c.read_f32::<LittleEndian>()?;
    let accel_x = c.read_f32::<LittleEndian>()?;
    let accel_y = c.read_f32::<LittleEndian>()?;
    let accel_z = c.read_f32::<LittleEndian>()?;
    let vel_x = c.read_f32::<LittleEndian>()?;
    let vel_y = c.read_f32::<LittleEndian>()?;
    let vel_z = c.read_f32::<LittleEndian>()?;
    skip(&mut c, 3)?; // AngularVelocity X/Y/Z
    skip(&mut c, 3)?; // Yaw/Pitch/Roll
    skip(&mut c, 4)?; // NormalizedSuspensionTravel FL/FR/RL/RR
    let tire_slip_ratio_fl = c.read_f32::<LittleEndian>()?;
    let tire_slip_ratio_fr = c.read_f32::<LittleEndian>()?;
    let tire_slip_ratio_rl = c.read_f32::<LittleEndian>()?;
    let tire_slip_ratio_rr = c.read_f32::<LittleEndian>()?;
    skip(&mut c, 4)?; // WheelRotationSpeed
    skip(&mut c, 4)?; // WheelOnRumbleStrip
    skip(&mut c, 4)?; // WheelInPuddleDepth
    skip(&mut c, 4)?; // SurfaceRumble
    let tire_slip_angle_fl = c.read_f32::<LittleEndian>()?;
    let tire_slip_angle_fr = c.read_f32::<LittleEndian>()?;
    let tire_slip_angle_rl = c.read_f32::<LittleEndian>()?;
    let tire_slip_angle_rr = c.read_f32::<LittleEndian>()?;
    skip(&mut c, 4)?; // TireCombinedSlip
    skip(&mut c, 4)?; // SuspensionTravelMeters
    let car_ordinal = c.read_i32::<LittleEndian>()?;
    let car_class = c.read_i32::<LittleEndian>()?;
    let car_pi = c.read_i32::<LittleEndian>()?;
    let drivetrain_type = c.read_i32::<LittleEndian>()?;
    let _num_cylinders = c.read_i32::<LittleEndian>()?;

    // Dash-only fields (bytes 232–310)
    skip(&mut c, 3)?; // Position X/Y/Z
    let speed_ms = c.read_f32::<LittleEndian>()?;
    let power = c.read_f32::<LittleEndian>()?;
    let torque = c.read_f32::<LittleEndian>()?;
    let tire_temp_fl = c.read_f32::<LittleEndian>()?;
    let tire_temp_fr = c.read_f32::<LittleEndian>()?;
    let tire_temp_rl = c.read_f32::<LittleEndian>()?;
    let tire_temp_rr = c.read_f32::<LittleEndian>()?;
    let boost = c.read_f32::<LittleEndian>()?;
    let fuel = c.read_f32::<LittleEndian>()?;
    let distance_traveled = c.read_f32::<LittleEndian>()?;
    let best_lap = c.read_f32::<LittleEndian>()?;
    let last_lap = c.read_f32::<LittleEndian>()?;
    let current_lap = c.read_f32::<LittleEndian>()?;
    let current_race_time = c.read_f32::<LittleEndian>()?;
    let lap_number = c.read_u16::<LittleEndian>()?;
    let race_position = c.read_u8()?;
    let throttle = c.read_u8()?;
    let brake = c.read_u8()?;
    let clutch = c.read_u8()?;
    let handbrake = c.read_u8()?;
    let gear = c.read_u8()?;
    let _steer = c.read_i8()?;
    let _driving_line = c.read_i8()?;
    let _ai_brake_diff = c.read_i8()?;
    // Now at byte 311

    // Optional tire wear (bytes 311+)
    let tire_wear_fl = if buf.len() >= 315 { Some(c.read_f32::<LittleEndian>()?) } else { None };
    let tire_wear_fr = if buf.len() >= 319 { Some(c.read_f32::<LittleEndian>()?) } else { None };
    let tire_wear_rl = if buf.len() >= 323 { Some(c.read_f32::<LittleEndian>()?) } else { None };
    let tire_wear_rr = if buf.len() >= 327 { Some(c.read_f32::<LittleEndian>()?) } else { None };

    Ok(TelemetryPacket {
        is_race_on,
        timestamp_ms,
        engine_max_rpm,
        engine_idle_rpm,
        current_engine_rpm,
        accel_x,
        accel_y,
        accel_z,
        vel_x,
        vel_y,
        vel_z,
        tire_slip_ratio_fl,
        tire_slip_ratio_fr,
        tire_slip_ratio_rl,
        tire_slip_ratio_rr,
        tire_slip_angle_fl,
        tire_slip_angle_fr,
        tire_slip_angle_rl,
        tire_slip_angle_rr,
        car_ordinal,
        car_class,
        car_pi,
        drivetrain_type,
        speed_ms,
        power,
        torque,
        tire_temp_fl,
        tire_temp_fr,
        tire_temp_rl,
        tire_temp_rr,
        boost,
        fuel,
        distance_traveled,
        best_lap,
        last_lap,
        current_lap,
        current_race_time,
        lap_number,
        race_position,
        throttle,
        brake,
        clutch,
        handbrake,
        gear,
        tire_wear_fl,
        tire_wear_fr,
        tire_wear_rl,
        tire_wear_rr,
    })
}

fn skip(c: &mut Cursor<&[u8]>, count: usize) -> std::io::Result<()> {
    let mut sink = vec![0u8; count * 4];
    c.read_exact(&mut sink)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_packet(len: usize) -> Vec<u8> {
        vec![0u8; len]
    }

    fn packet_with_speed(speed_ms: f32) -> Vec<u8> {
        let mut buf = zero_packet(311);
        buf[244..248].copy_from_slice(&speed_ms.to_le_bytes());
        buf
    }

    #[test]
    fn rejects_short_packet() {
        let buf = zero_packet(100);
        assert!(parse(&buf).is_err());
    }

    #[test]
    fn parses_speed_field() {
        let buf = packet_with_speed(44.44);
        let pkt = parse(&buf).unwrap();
        assert!((pkt.speed_ms - 44.44).abs() < 0.001);
    }

    #[test]
    fn accepts_311_byte_packet() {
        let buf = zero_packet(311);
        assert!(parse(&buf).is_ok());
    }

    #[test]
    fn accepts_324_byte_packet_with_tire_wear() {
        let mut buf = zero_packet(324);
        buf[311..315].copy_from_slice(&0.85f32.to_le_bytes());
        let pkt = parse(&buf).unwrap();
        assert!((pkt.tire_wear_fl.unwrap() - 0.85).abs() < 0.001);
    }

    #[test]
    fn is_race_on_zero_parses_as_false() {
        let buf = zero_packet(311);
        let pkt = parse(&buf).unwrap();
        assert!(!pkt.is_race_on);
    }

    #[test]
    fn is_race_on_one_parses_as_true() {
        let mut buf = zero_packet(311);
        buf[0..4].copy_from_slice(&1i32.to_le_bytes());
        let pkt = parse(&buf).unwrap();
        assert!(pkt.is_race_on);
    }
}
