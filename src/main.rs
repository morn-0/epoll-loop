use std::{collections::HashMap, error::Error, net::TcpListener, os::fd::AsRawFd, ptr};

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    listener.set_nonblocking(true)?;

    let epoll = unsafe { libc::epoll_create1(0) };
    unsafe {
        libc::epoll_ctl(
            epoll,
            libc::EPOLL_CTL_ADD,
            listener.as_raw_fd(),
            &mut libc::epoll_event {
                events: (libc::EPOLLIN | libc::EPOLLONESHOT) as u32,
                u64: 1,
            },
        );
    }

    let mut begin_key = 1;
    let mut connects = HashMap::new();
    let mut events: Vec<libc::epoll_event> = Vec::with_capacity(1024);

    loop {
        events.clear();

        unsafe {
            let len = libc::epoll_wait(
                epoll,
                events.as_mut_ptr(),
                events.capacity() as i32,
                1000 * 5,
            );
            events.set_len(len as usize);
        }

        for event in &events {
            match event.u64 {
                1 => {
                    let connect = listener.accept()?;
                    connect.0.set_nonblocking(true)?;
                    begin_key += 1;

                    unsafe {
                        libc::epoll_ctl(
                            epoll,
                            libc::EPOLL_CTL_ADD,
                            connect.0.as_raw_fd(),
                            &mut libc::epoll_event {
                                events: (libc::EPOLLIN | libc::EPOLLONESHOT) as u32,
                                u64: begin_key,
                            },
                        );
                    }
                    connects.insert(begin_key, connect);

                    unsafe {
                        libc::epoll_ctl(
                            epoll,
                            libc::EPOLL_CTL_MOD,
                            listener.as_raw_fd(),
                            &mut libc::epoll_event {
                                events: (libc::EPOLLIN | libc::EPOLLONESHOT) as u32,
                                u64: 1,
                            },
                        );
                    }
                }
                key => {
                    if let Some(connect) = connects.get(&key) {
                        match event.events as i32 {
                            v if v & libc::EPOLLIN == libc::EPOLLIN => {
                                println!("read {:?}", connect.1);

                                unsafe {
                                    libc::epoll_ctl(
                                        epoll,
                                        libc::EPOLL_CTL_MOD,
                                        connect.0.as_raw_fd(),
                                        &mut libc::epoll_event {
                                            events: (libc::EPOLLOUT | libc::EPOLLONESHOT) as u32,
                                            u64: key,
                                        },
                                    );
                                }
                            }
                            v if v & libc::EPOLLOUT == libc::EPOLLOUT => {
                                println!("write {:?}", connect.1);

                                unsafe {
                                    libc::epoll_ctl(
                                        epoll,
                                        libc::EPOLL_CTL_DEL,
                                        connect.0.as_raw_fd(),
                                        ptr::null_mut(),
                                    );
                                }

                                connects.remove(&key);
                            }
                            _ => {
                                eprintln!("error");
                            }
                        }
                    }
                }
            }
        }
    }
}
