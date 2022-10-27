variable "vpc_cidr" {
  type        = string
  default     = "10.0.0.0/16"
  description = "CIDR for VPC"
}

variable "tenancy" {
  type        = string
  default     = "default"
  description = ""
}

variable "enable_dns_support" {
    default = true
}

variable "enable_dns_hostnames" {
    default = true
}

variable "vpc_name" {
  type        = string
  default     = "main"
  description = ""
}

variable "public_cidr" {}
variable "private_cidr" {}

// dublication between variables.tf and modules_variables.tf
variable "availability_zone" {
  default = "eu-north-1b"
}

