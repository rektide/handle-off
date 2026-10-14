#![cfg(target_os = "linux")]
use handle_off_state_memfd::{BorrowedImage, MAX_BYTES, OwnedImage};
use std::{
    fs::File,
    io::{Seek, SeekFrom},
    os::fd::AsFd,
};

#[test]
fn sealed_bytes_reject_mutation_and_reads_preserve_shared_offset() {
    let image = OwnedImage::create(b"immutable bytes", 1024).unwrap();
    assert_eq!(image.len(), 15);
    assert!(
        rustix::io::fcntl_getfd(&image)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC)
    );
    let mut file = File::from(image.as_fd().try_clone_to_owned().unwrap());
    file.seek(SeekFrom::Start(5)).unwrap();
    assert_eq!(image.borrow().read_all().unwrap(), b"immutable bytes");
    assert_eq!(image.read_range(10, 5).unwrap(), b"bytes");
    assert_eq!(file.stream_position().unwrap(), 5);
    assert!(rustix::io::pwrite(&image, b"replace", 0).is_err());
    assert!(rustix::fs::ftruncate(&image, 1).is_err());
    assert!(rustix::fs::ftruncate(&image, 100).is_err());
    let copy = image.try_clone_to_owned().unwrap();
    drop(image);
    assert_eq!(copy.read_all().unwrap(), b"immutable bytes");
}
#[test]
fn limits_seals_adoption_and_ranges_are_checked() {
    assert!(OwnedImage::create(b"large", 4).is_err());
    assert!(OwnedImage::create(b"", MAX_BYTES + 1).is_err());
    let empty = OwnedImage::create(b"", 0).unwrap();
    assert!(empty.is_empty());
    assert!(empty.read_all().unwrap().is_empty());
    let image = OwnedImage::create(b"abc", 3).unwrap();
    assert!(image.read_range(u64::MAX, 1).is_err());
    assert!(image.read_range(2, 2).is_err());
    assert!(BorrowedImage::from_borrowed(image.as_fd(), 2).is_err());
    assert_eq!(
        BorrowedImage::from_borrowed(image.as_fd(), 3)
            .unwrap()
            .read_all()
            .unwrap(),
        b"abc"
    );
    assert!(OwnedImage::adopt(image.try_clone_to_owned().unwrap().into_fd(), 2).is_err());
    assert_eq!(image.read_all().unwrap(), b"abc");
    let fd = rustix::fs::memfd_create(c"unsealed", rustix::fs::MemfdFlags::ALLOW_SEALING).unwrap();
    assert!(BorrowedImage::from_borrowed(fd.as_fd(), 10).is_err());
    rustix::fs::fcntl_add_seals(
        &fd,
        rustix::fs::SealFlags::GROW | rustix::fs::SealFlags::SHRINK | rustix::fs::SealFlags::SEAL,
    )
    .unwrap();
    // FUTURE_WRITE/size seals alone do not exclude existing writable mappings.
    assert!(OwnedImage::adopt(fd, 10).is_err());
    assert!(OwnedImage::adopt(File::open("/dev/null").unwrap().into(), 10).is_err());
    let fd = rustix::fs::memfd_create(c"adopt", rustix::fs::MemfdFlags::ALLOW_SEALING).unwrap();
    assert!(
        !rustix::io::fcntl_getfd(&fd)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC)
    );
    rustix::fs::fcntl_add_seals(
        &fd,
        rustix::fs::SealFlags::WRITE
            | rustix::fs::SealFlags::GROW
            | rustix::fs::SealFlags::SHRINK
            | rustix::fs::SealFlags::SEAL,
    )
    .unwrap();
    let adopted = OwnedImage::adopt(fd, 0).unwrap();
    assert!(
        rustix::io::fcntl_getfd(&adopted)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC)
    );
}
