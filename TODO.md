# To do

There's a lot to do to make this useful.

First of all, I'd like to get this working as a Rust library. This is not the
end goal, but keeping it as a library makes it easier for me to get started.

Once the library is useful, I will start working on a command-line interface
that manages the daemon.

Expected functionality is as follows:

## Process management

- [x] start processes
- [ ] wait for a process to start, and check it hasn't immediately crashed
- [ ] stop all processes on shutdown
- [ ] provide a way to shut down a single process
- [x] time out when waiting for a process to stop, and kill it
- [ ] log when a process stops with a non-zero exit code
- [ ] log when a process stops with a signal exit code
- [ ] log when a process has been killed
- [ ] detect when a process has stopped, and log it
- [ ] group processes, and shut down entire process groups
- [ ] capture the `PATH` from the client, not the daemon
- [ ] sanitize all environment variables except those specified

## TCP ports

- [x] wait for a service to start on a given TCP port
- [ ] time out responsibly when waiting for a port to open up
- [ ] provide a free port to be used

## Output

- [ ] capture service output to a file, and print it on demand
- [ ] optionally keep output around after shutting down the service

## Health checks

- [ ] optionally, restart on crash
- [ ] recognize when a service is unresponsive, and restart
- [ ] configurable retries

## Logging

- [ ] ensure that exceptions are logged without crashing the daemon
- [ ] log in a structured format, with error codes, severity, and timestamps
- [ ] serialize the logs so they're pretty on the terminal
- [ ] report meaningful errors back to the client

## Responsiveness

- [ ] never block without a timeout
- [ ] ensure that timeouts are always configurable, and not hard-coded
- [ ] log when things are taking a while

## Command-line interface

- [x] explicitly start the daemon
- [ ] initialize the daemon on first use
- [ ] shut down the daemon
- [ ] start a service
- [ ] stop a service
- [ ] natural syntax for waiting, health checks, and restart policies

## Scoping

- [ ] services scoped to a shell or other parent process
- [ ] capture sanitized environment variables when creating the scope
- [ ] shut down services automatically when out of scope
- [ ] shut down the daemon automatically when everything is out of scope
