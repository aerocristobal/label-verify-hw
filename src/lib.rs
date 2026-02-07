//! TTB Label Verification System
//!
//! This library provides the core functionality for the label-verify-hw system,
//! which performs automated verification of beverage labels using Cloudflare
//! Workers AI and R2 storage.

pub mod app_state;
pub mod config;
pub mod db;
pub mod models;
pub mod routes;
pub mod services;
