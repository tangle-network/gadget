//! Utilities for spinning up and managing Docker containers
//!
//! This module provides wrappers around [`bollard`] to simplify Docker interactions within blueprints.

pub use bollard;
use bollard::container::{
    Config, CreateContainerOptions, StartContainerOptions, StopContainerOptions,
    WaitContainerOptions,
};
use bollard::models::{ContainerCreateResponse, HostConfig};
use bollard::{Docker, API_DEFAULT_VERSION};
use std::sync::Arc;
use subxt::ext::futures::{Stream, StreamExt};

/// A [Docker](https://en.wikipedia.org/wiki/Docker_(software)) container
#[derive(Debug)]
pub struct Container<'a> {
    id: Option<String>,
    image: String,
    connection: &'a Docker,
    options: ContainerOptions,
}

#[derive(Debug, Default, Clone)]
struct ContainerOptions {
    env: Option<Vec<String>>,
    cmd: Option<Vec<String>>,
    binds: Option<Vec<String>>,
}

impl<'a> Container<'a> {
    /// Create a new `Container`
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::docker::{connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     // We can now start our container
    ///     container.start(true).await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new<T>(connection: &'a Docker, image: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            id: None,
            image: image.into(),
            connection,
            options: ContainerOptions::default(),
        }
    }

    /// Set the environment variables for the container
    ///
    /// NOTE: This will override any existing variables.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::docker::{connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     container.env(["FOO=BAR", "BAZ=QUX"]);
    ///
    ///     // We can now start our container, and the "FOO" and "BAZ" env vars will be set
    ///     container.start(true).await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn env(&mut self, env: impl IntoIterator<Item = impl Into<String>>) -> &mut Self {
        self.options.env = Some(env.into_iter().map(Into::into).collect());
        self
    }

    /// Set the command to run
    ///
    /// The command is provided as a list of strings.
    ///
    /// NOTE: This will override any existing command
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::docker::{connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     container.cmd(["echo", "Hello!"]);
    ///
    ///     // We can now start our container, and the command "echo Hello!" will run
    ///     container.start(true).await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn cmd(&mut self, cmd: impl IntoIterator<Item = impl Into<String>>) -> &mut Self {
        self.options.cmd = Some(cmd.into_iter().map(Into::into).collect());
        self
    }

    /// Set a list of volume binds
    ///
    /// These binds are in the standard `host:dest[:options]` format. For more information, see
    /// the [Docker documentation](https://docs.docker.com/engine/storage/bind-mounts/).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::docker::{connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     // Mount './my-host-dir' at '/some/container/dir' and make it read-only
    ///     container.binds(["./my-host-dir:/some/container/dir:ro"]);
    ///
    ///     // We can now start our container
    ///     container.start(true).await?;
    ///     Ok(())
    /// }
    /// ```
    pub fn binds(&mut self, binds: impl IntoIterator<Item = impl Into<String>>) -> &mut Self {
        self.options.binds = Some(binds.into_iter().map(Into::into).collect());
        self
    }

    /// Get the container ID if it has been created
    ///
    /// This will only have a value if [`Container::create`] or [`Container::start`] has been
    /// called prior.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Attempt to create the container
    ///
    /// This will take the following into account:
    ///
    /// * [`Container::env`]
    /// * [`Container::cmd`]
    /// * [`Container::binds`]
    ///
    /// Be sure to set these before calling this!
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::docker::{connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     container.env(["FOO=BAR", "BAZ=QUX"]);
    ///     container.cmd(["echo", "Hello!"]);
    ///     container.binds(["./host-data:/container-data"]);
    ///
    ///     // The container is created using the above settings
    ///     container.create().await?;
    ///
    ///     // Now it can be started
    ///     container.start(true).await?;
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument]
    pub async fn create(&mut self) -> Result<(), bollard::errors::Error> {
        crate::debug!("Creating container");

        let config = Config {
            image: Some(self.image.clone()),
            cmd: self.options.cmd.clone(),
            attach_stdout: Some(true),
            host_config: Some(HostConfig {
                binds: self.options.binds.clone(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let ContainerCreateResponse { id, warnings } = self
            .connection
            .create_container(None::<CreateContainerOptions<String>>, config)
            .await?;
        for warning in warnings {
            crate::warn!("{}", warning);
        }

        self.id = Some(id);
        Ok(())
    }

    /// Attempt to start the container
    ///
    /// NOTE: If the container has not yet been created, this will attempt to call [`Container::create`] first.
    ///
    /// `wait_for_exit` will wait for the container to exit before returning.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::docker::{connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     container.cmd(["echo", "Hello!"]);
    ///
    ///     // We can now start our container, and the command "echo Hello!" will run.
    ///     let wait_for_exit = true;
    ///     container.start(wait_for_exit).await?;
    ///
    ///     // Since we waited for the container to exit, we don't have to stop it.
    ///     // It can now just be removed.
    ///     container.remove(None).await?;
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument]
    pub async fn start(&mut self, wait_for_exit: bool) -> Result<(), bollard::errors::Error> {
        if self.id.is_none() {
            self.create().await?;
        }

        crate::debug!("Starting container");
        let id = self.id.as_ref().unwrap();
        self.connection
            .start_container(id, None::<StartContainerOptions<String>>)
            .await?;

        if wait_for_exit {
            self.wait().await?;
        }

        Ok(())
    }

    /// Stop a running container
    ///
    /// NOTE: It is not an error to call this on a container that has not been started,
    ///       it will simply do nothing.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::docker::{connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     // Does nothing, the container isn't started
    ///     container.stop().await?;
    ///
    ///     // Stops the running container
    ///     container.start(false).await?;
    ///     container.stop().await?;
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument]
    pub async fn stop(&mut self) -> Result<(), bollard::errors::Error> {
        let Some(id) = &self.id else {
            crate::warn!("Container not started");
            return Ok(());
        };

        self.connection
            .stop_container(id, None::<StopContainerOptions>)
            .await?;

        Ok(())
    }

    /// Remove a container
    ///
    /// NOTE: To remove a running container, a [`RemoveContainerOptions`] must be provided
    ///       with the `force` flag set.
    ///
    /// See also: [`bollard::container::RemoveContainerOptions`]
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::docker::{bollard, connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     // Start our container
    ///     container.start(false).await?;
    ///
    ///     let remove_container_options = bollard::container::RemoveContainerOptions {
    ///         force: true,
    ///         ..Default::default()
    ///     };
    ///
    ///     // Kills the container and removes it
    ///     container.remove(Some(remove_container_options)).await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`RemoveContainerOptions::force`]: bollard::container::RemoveContainerOptions::force
    #[tracing::instrument]
    pub async fn remove(
        mut self,
        options: Option<bollard::container::RemoveContainerOptions>,
    ) -> Result<(), bollard::errors::Error> {
        let Some(id) = self.id.take() else {
            crate::warn!("Container not started");
            return Ok(());
        };

        self.connection.remove_container(&id, options).await?;
        Ok(())
    }

    /// Wait for a container to exit
    ///
    /// NOTE: It is not an error to call this on a container that has not been started,
    ///       it will simply do nothing.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_sdk::docker::{bollard, connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     // Start our container
    ///     container.start(false).await?;
    ///
    ///     // Once this returns, we know that the container has exited.
    ///     container.wait().await?;
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument]
    pub async fn wait(&self) -> Result<(), bollard::errors::Error> {
        let Some(id) = &self.id else {
            crate::warn!("Container not created");
            return Ok(());
        };

        wait_for_container(self.connection, id).await?;
        Ok(())
    }

    /// Fetch the container log stream
    ///
    /// NOTE: It is not an error to call this on a container that has not been started,
    ///       it will simply do nothing and return `None`.
    ///
    /// See also:
    ///
    /// * [`bollard::container::LogsOptions`]
    /// * [`bollard::container::LogOutput`]
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use futures::StreamExt;
    /// use gadget_sdk::docker::{bollard, connect_to_docker, Container};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), gadget_sdk::Error> {
    ///     let connection = connect_to_docker(None).await?;
    ///     let mut container = Container::new(&connection, "rustlang/rust");
    ///
    ///     // Start our container and wait for it to exit
    ///     container.start(true).await?;
    ///
    ///     // We want to collect logs from stderr
    ///     let logs_options = bollard::container::LogsOptions {
    ///         stderr: true,
    ///         follow: true,
    ///         ..Default::default()
    ///     };
    ///
    ///     // Get our log stream
    ///     let mut logs = container
    ///         .logs(Some(logs_options))
    ///         .await
    ///         .expect("logs should be present");
    ///
    ///     // Now we want to print anything from stderr
    ///     while let Some(Ok(out)) = logs.next().await {
    ///         if let bollard::container::LogOutput::StdErr { message } = out {
    ///             eprintln!("Uh oh! Something was written to stderr: {:?}", message);
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument]
    pub async fn logs(
        &self,
        logs_options: Option<bollard::container::LogsOptions<String>>,
    ) -> Option<impl Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>>>
    {
        let Some(id) = &self.id else {
            crate::warn!("Container not created");
            return None;
        };

        Some(self.connection.logs(id, logs_options))
    }
}

/// Connect to a local [Docker] socket
///
/// NOTE: If `socket` is `None`, this will attempt to connect to `/var/run/docker.sock`.
///
/// # Examples
///
/// ```rust,no_run
/// use gadget_sdk::docker::connect_to_docker;
///
/// #[tokio::main]
/// async fn main() -> Result<(), gadget_sdk::Error> {
///     let connection = connect_to_docker(None).await?;
///     
///     // I now have a handle to my local Docker server!
///     Ok(())
/// }
/// ```
///
/// [Docker]: https://en.wikipedia.org/wiki/Docker_(software)
pub async fn connect_to_docker(
    socket: Option<&str>,
) -> Result<Arc<Docker>, bollard::errors::Error> {
    crate::info!("Connecting to local docker server...");
    let docker = Docker::connect_with_socket(
        socket.unwrap_or("/var/run/docker.sock"),
        120,
        API_DEFAULT_VERSION,
    )?;
    if let Err(e) = docker.ping().await {
        crate::error!("Failed to ping docker server: {}", e);
        return Err(e);
    }

    Ok(Arc::new(docker))
}

async fn wait_for_container(docker: &Docker, id: &str) -> Result<(), bollard::errors::Error> {
    let options = WaitContainerOptions {
        condition: "not-running",
    };

    let mut wait_stream = docker.wait_container(id, Some(options));

    while let Some(msg) = wait_stream.next().await {
        match msg {
            Ok(msg) => {
                if msg.status_code == 0 {
                    break;
                }

                if let Some(err) = msg.error {
                    crate::error!("Failed to wait for container: {:?}", err.message);
                    // TODO: These aren't the same error type, is this correct?
                    return Err(bollard::errors::Error::DockerContainerWaitError {
                        error: err.message.unwrap_or_default(),
                        code: msg.status_code,
                    });
                }
            }
            Err(e) => {
                match &e {
                    bollard::errors::Error::DockerContainerWaitError { error, code } => {
                        crate::error!("Container failed with status code `{}`: {error}", code);
                    }
                    _ => crate::error!("Container failed with error: {:?}", e),
                }
                return Err(e);
            }
        }
    }

    Ok(())
}
