use clap::{value_parser, ArgMatches};

fn open_blkdev_by_path(path: &str) -> Result<libblkid_rs::BlkidDevno, Box<dyn std::error::Error>> {
    let probe = libblkid_rs::BlkidProbe::new_from_filename(std::path::Path::new(&path))?;
    return Ok(probe.get_devno());
}

fn find_and_open_blkdev_by_token(token: &str) -> Result<Option<libblkid_rs::BlkidDevno>, Box<dyn std::error::Error>> {
    let mut cache = libblkid_rs::BlkidCache::get_cache(Some(std::path::Path::new("/dev/null")))?;
    cache.probe_all()?;
    let dev = cache.get_devname(either::Either::Left(token))?;
    eprintln!("Dev: {}", dev);
    //dbg!(dev);

    let blkdev = open_blkdev_by_path(dev.as_str())?;
    return Ok(Some(blkdev));
}

fn find_partition_by_number(parent: &libblkid_rs::BlkidDevno, number: i32) -> Result<libblkid_rs::BlkidDevno, Box<dyn std::error::Error>> {
    let devname = parent.to_devname()?;

    // if devname ends with a number, we need to add "p", e.g. mmcblk0p1 vs. sda1
    let mut part_devname = devname.clone();
    if devname.chars().last().unwrap().is_numeric() {
        part_devname.push('p');
    }

    part_devname.push_str(&number.to_string());
    let blkdev = open_blkdev_by_path(part_devname.as_str())?;

    return Ok(blkdev);
}

pub fn find_blkdev_name_by_serial_via_sysfs(
    sysfs_mount: &str,
    serial: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // Construct the path to the /sys/block directory.
    let block_devices_path = std::path::Path::new(sysfs_mount).join("block");

    // Check if the /sys/block path exists and is a directory.
    if !block_devices_path.is_dir() {
        return Err(format!("'{}' is not a valid directory.", block_devices_path.display()).into());
    }

    // Iterate over each entry in the /sys/block directory.
    for entry in std::fs::read_dir(block_devices_path)? {
        let entry = entry?;
        let dev_path = entry.path();

        // We are only interested in directories (which represent block devices).
        if dev_path.is_dir() {
            // Construct the path to the potential 'serial' file for this device.
            let serial_path = dev_path.join("serial");

            // Check if the 'serial' file actually exists for this device.
            // Not all block devices expose a serial number this way.
            if serial_path.is_file() {
                // Read the serial number from the file. The content might have trailing whitespace.
                let file_serial = std::fs::read_to_string(&serial_path)?;

                // Compare the trimmed serial from the file with the target serial.
                if file_serial.trim() == serial {
                    // If they match, get the directory name, which is the device name.
                    if let Some(dev_name) = dev_path.file_name() {
                        if let Some(dev_name_str) = dev_name.to_str() {
                            // Found a match, return the device name.
                            return Ok(Some(dev_name_str.to_string()));
                        }
                    }
                }
            }
        }
    }

    // If the loop completes without finding a match, return None.
    Ok(None)
}

fn find_and_open_blkdev_by_serial(serial: &str) -> Result<Option<libblkid_rs::BlkidDevno>, Box<dyn std::error::Error>> {
    let maybe_devname = find_blkdev_name_by_serial_via_sysfs("/sys", serial)?;

    match maybe_devname {
        Some(devname) => {
            eprintln!("Devname: {}", devname);
            let blkdev = open_blkdev_by_path(format!("/dev/{}", devname).as_str())?;
            return Ok(Some(blkdev));
        }
        None => {
            eprintln!("Device with serial {} not found", serial);
            return Ok(None);
        }
    }
}

fn cli() -> clap::Command {
    clap::Command::new("isak")
        .about("Initramfs Swiss Army Knife")
        .subcommand_required(true)
        .subcommand(
            clap::Command::new("blkdev")
            .about("block device operations")
            .subcommand_required(true)
            .subcommand(
                clap::Command::new("find")
                .about("find block device")
                .arg(
                    clap::arg!(--token <TOKEN> "Token to search for")
                )
                .arg(
                    clap::arg!(--parent "Search for parent device")
                )
                .arg(
                    clap::arg!(--device <DEVICE> "Device to search for")
                )
                .arg(
                    clap::arg!(--partno <PART> "Partition to search for")
                    .value_parser(value_parser!(i32))
                )
                .arg(
                    clap::arg!(--serial <SERIAL> "serial number to search for")
                )
            )
        )
}

fn blkdev_find(find_cmd: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let mut maybe_blkdev: Option<libblkid_rs::BlkidDevno> = None;

    let serial = find_cmd.get_one::<String>("serial");

    match serial {
        Some(s) => {
            eprintln!("Serial: {}", s);
            maybe_blkdev = find_and_open_blkdev_by_serial(s.as_str())?;
        }
        _ => {
            eprintln!("Serial not found");
        }
    }

    //let token = find_cmd.value_of("token").unwrap();
    let token = find_cmd.get_one::<String>("token");

    match token {
        Some(t) => {
            eprintln!("Token: {}", t);
            maybe_blkdev = find_and_open_blkdev_by_token(t.as_str())?;
        }
        _ => {
            eprintln!("Token not found");
        }
    }

    let device = find_cmd.get_one::<String>("device");

    match device {
        Some(d) => {
            eprintln!("Device: {}", d);

            let blkdev = open_blkdev_by_path(d.as_str())?;
            maybe_blkdev = Some(blkdev);
        }
        _ => {
            eprintln!("Device not found");
        }
    }

    let parent = *find_cmd.get_one::<bool>("parent").unwrap();
    eprintln!("Parent: {}", parent);

    if maybe_blkdev.is_none() {
        eprintln!("Block device not found");
        // return Ok(());
        return Err("Block device not found".into());
    }

    let mut blkdev = maybe_blkdev.unwrap();

    if parent == true {
        eprintln!("Parent");

        let (_name, parent_blkdev) = blkdev.to_wholedisk()?;
        blkdev = parent_blkdev;
    }

    let partition_number = find_cmd.get_one::<i32>("partno");

    match partition_number {
        Some(p) => {
            eprintln!("Partition number: {}", p);

            let partition_blkdev = find_partition_by_number(&blkdev, *p)?;
            blkdev = partition_blkdev;
        },
        _ => {
            // eprintln!("Partition number not found");
        }
    }

    println!("{}", blkdev.to_devname()?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = cli().get_matches();
    //dbg!(matches);

    match matches.subcommand() {
        Some(("blkdev", blkdev_cmd)) => {

            match blkdev_cmd.subcommand() {
                Some(("find", find_cmd)) => {
                    
                    blkdev_find(find_cmd)?;
                }
                _ => {
                    return Err("Unknown command".into());
                }
            }
        }
        _ => {
            return Err("Unknown command".into());
        }
    }

    return Ok(())
}
