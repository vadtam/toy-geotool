variable "aws_region" {
    default = "eu-north-1"
}

variable "ec2_count" {
  default = "1"
}

variable "ami_id" {
    // Debian 11 (HVM), SSD Volume Type in eu-north-1
    default = "ami-00189fd46154b0f9d"
}

variable "instance_type" {
  default = "t3.medium"
}

variable "domain_name" {
  default = "geotool.cloud"
}

variable "project" {
  default = "Geotool"
}

// dublication between variables.tf and modules_variables.tf
variable "availability_zone" {
  default = "eu-north-1b"
}

