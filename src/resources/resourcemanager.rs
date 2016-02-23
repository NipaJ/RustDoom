use std::fs::File;
use std::path::Path;
use std::io;
use std::io::Read;
use std::io::BufReader;
use std::io::SeekFrom;
use std::io::Seek;
use resources::bsp;
use resources::WadPackage;
use resources::WadResult;
use resources::WadError;

pub type PackageLoadResult<T> = Result<T, PackageLoadError>;

enum PackageFormat {
    Unknown,
    IWad,
    PWad
}

#[derive(Debug)]
pub enum PackageLoadError {
    UnknownPackage,
    IoFailure(io::Error),
    WadError(WadError)
}

pub struct ResourceManager {
    maps : Vec<bsp::Map>
}

impl ResourceManager {
    pub fn new() -> ResourceManager {
        ResourceManager {
            maps: Vec::<bsp::Map>::new()
        }
    }

    pub fn find_map(&self, name : &str) -> Option<&bsp::Map> {
        for level in &self.maps {
            if level.name == name {
                return Some(level);
            }
        }
        return None;
    }

    pub fn load_package<P : AsRef<Path>>(&mut self, path : P) -> PackageLoadResult<()> {
        let mut file = match File::open(path) {
            Ok(value) => value,
            Err(err) => return Err(PackageLoadError::IoFailure(err))
        };

        match try!(detect_package_format(&mut file)) {
            PackageFormat::IWad | PackageFormat::PWad => {
                self.add_package(&try!(wrap_wad_error(WadPackage::new(&mut file))));
            },
            _ => return Err(PackageLoadError::UnknownPackage)
        }

        Ok(())
    }

    pub fn clear_resources(&mut self) {
        self.maps.clear();
    }

    fn add_package(&mut self, package : &WadPackage) {
        self.maps.extend_from_slice(package.get_maps());
    }
}

fn detect_package_format(file : &mut File) -> PackageLoadResult<PackageFormat> {
    match file.seek(SeekFrom::Start(0)) {
        Err(error) => return Err(PackageLoadError::IoFailure(error)),
        _ => ()
    }

    let mut reader = BufReader::new(file);

    // Check for an IWAD or PWAD signature
    let mut signature = [0u8; 4];
    if let Err(error) = reader.read_exact(&mut signature[..]) {
        return Err(PackageLoadError::IoFailure(error));
    }

    if signature[1] == 'W' as u8 && signature[2] == 'A' as u8 && signature[3] == 'D' as u8 {
        if signature[0] == 'I' as u8 {
            return Ok(PackageFormat::IWad);
        }
        else if signature[0] == 'P' as u8 {
            return Ok(PackageFormat::PWad);
        }
    }

    Ok(PackageFormat::Unknown)
}

fn wrap_wad_error<T>(error : WadResult<T>) -> PackageLoadResult<T> {
    match error {
        Ok(value) => Ok(value),
        Err(WadError::IoFailure(error)) => Err(PackageLoadError::IoFailure(error)),
        Err(error) => Err(PackageLoadError::WadError(error))
    }
}
