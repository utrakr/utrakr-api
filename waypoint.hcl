project = "utrakr"

app "utrakr-api" {
  labels = {
    "service" = "utrakr-api",
  }

  build {
    use "docker" {}

    registry {
      use "docker" {
        image = "us.gcr.io/utrakr/utrakr-api"
        tag   = gitrefpretty()
      }
    }
  }

  deploy {
    use "docker" {
      service_port = 8080
    }
  }
}
