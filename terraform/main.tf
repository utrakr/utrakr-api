provider "google" {
  project = "utrakr"
  region  = "us-central1"
  version = "~> 3.20"
}

terraform {
  required_version = "~> 0.12"
}

terraform {
  required_version = "~> 0.12"
  backend "gcs" {
    bucket = "utrakr-all-terraform-state"
    prefix = "utrakr-api"
  }
}

data "terraform_remote_state" "crit_dns" {
  backend = "gcs"

  config = {
    bucket = "utrakr-all-terraform-state"
    prefix = "crit-dns"
  }
}

data "terraform_remote_state" "vpc" {
  backend = "gcs"

  config = {
    bucket = "utrakr-all-terraform-state"
    prefix = "vpc"
  }
}

data "terraform_remote_state" "homepage" {
  backend = "gcs"

  config = {
    bucket = "utrakr-all-terraform-state"
    prefix = "homepage"
  }
}

data "google_dns_managed_zone" "root" {
  name = data.terraform_remote_state.crit_dns.outputs["root_zone_name"]
}

locals {
  app      = "utrakr-api"
  location = data.terraform_remote_state.vpc.outputs["cloud_functions_connector_region"]
}