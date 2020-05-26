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

provider "google" {
  project = "utrakr"
  region  = "us-central1"
  version = "~> 3.20"
}

provider "random" {
  version = "~> 2.2"
}

provider "template" {
  version = "~> 2.1"
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
  name = data.terraform_remote_state.crit_dns.outputs["google_dns_managed_zone_root"].name
}

data "google_compute_network" "default" {
  name = data.terraform_remote_state.vpc.outputs["google_compute_network_default"].name
}

data "google_compute_subnetwork" "default" {
  name   = data.terraform_remote_state.vpc.outputs["google_compute_subnetwork_default"].name
  region = data.terraform_remote_state.vpc.outputs["google_compute_subnetwork_default"].region
}

locals {
  app = "utrakr-api"
}