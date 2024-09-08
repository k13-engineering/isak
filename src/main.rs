use clap::ArgMatches;

fn open_blkdev_by_path(path: &str) -> Result<libblkid_rs::BlkidDevno, Box<dyn std::error::Error>> {
    let probe = libblkid_rs::BlkidProbe::new_from_filename(std::path::Path::new(&path))?;
    return Ok(probe.get_devno());
}

fn find_and_open_blkdev_by_token(token: &str) -> Result<Option<libblkid_rs::BlkidDevno>, Box<dyn std::error::Error>> {
    let mut cache = libblkid_rs::BlkidCache::get_cache(None)?;
    cache.probe_all()?;
    let dev = cache.get_devname(either::Either::Left(token))?;
    eprintln!("Dev: {}", dev);
    //dbg!(dev);

    let blkdev = open_blkdev_by_path(dev.as_str())?;
    return Ok(Some(blkdev));
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
            )
        )
}

fn blkdev_find(find_cmd: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let mut maybe_blkdev: Option<libblkid_rs::BlkidDevno> = None;

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
