use std::io::Result;

use address::Address;
use mem::PhysicalRead;

pub fn vtop<T: PhysicalRead>(mem: &mut T, dtb: Address, addr: Address) -> Result<Address> {
    println!("x86_vtop() not implemented yet");
    Ok(Address::from(0))
}