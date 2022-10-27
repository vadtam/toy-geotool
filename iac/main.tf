provider "aws" {
    region = var.aws_region
}

module "vpc" {
    source = "./modules"
    vpc_name = "geotool"
    vpc_cidr = "192.168.0.0/16"
    public_cidr = "192.168.1.0/24"
    private_cidr = "192.168.2.0/24"
}

variable "geotool_ssh_public_key" {
   type=string
}

resource "aws_key_pair" "geotool-ssh-key" {
  key_name   = "geotool-ssh-key"
  public_key = var.geotool_ssh_public_key
}

resource "aws_instance" "public-ec2" {
  ami           = var.ami_id
  availability_zone = var.availability_zone
  credit_specification {
     cpu_credits = "standard"
  }
  ebs_optimized = false
  instance_type = var.instance_type
  subnet_id     = module.vpc.subnet_public_id
  root_block_device {
     volume_type = "standard"
     delete_on_termination = true
     encrypted = false
     tags = {
       Name = "${var.project} Root"
     }
     volume_size = 8
  }
  ebs_block_device {
     device_name = "/dev/sdf"
     volume_type = "standard"
     delete_on_termination = true
     encrypted = false
     tags = {
       Name = "${var.project} Postgres"
     }
     volume_size = 100
  }
  key_name      = "geotool-ssh-key"
  vpc_security_group_ids = [ aws_security_group.ec2-sg.id ]
  associate_public_ip_address = true

  tags = {
    Name = var.project
  }

  depends_on = [ module.vpc.vpc_id, module.vpc.igw_id ]

  user_data = <<EOF
#!/bin/sh
sudo apt-get update
sudo apt-get install -y mysql-server
EOF
}

resource "aws_security_group" "ec2-sg" {
  name        = "security-group"
  description = "allow inbound access to the Application task from NGINX"
  vpc_id      = module.vpc.vpc_id
  /* 
  ingress {
    protocol    = "tcp"
    from_port   = 22
    to_port     = 22
    cidr_blocks = [ "0.0.0.0/0" ]
  }*/
  /*   
  ingress {
    protocol    = "tcp"
    from_port   = 80
    to_port     = 80
    cidr_blocks = [ "0.0.0.0/0" ]
  }
  */
   
  ingress {
    protocol    = "tcp"
    from_port   = 443
    to_port     = 443
    cidr_blocks = [ "0.0.0.0/0" ]
  }
  
  ingress {
    protocol    = "icmp"
    from_port   = -1
    to_port     = -1
    cidr_blocks = [ "0.0.0.0/0" ]
  }

  egress {
    protocol    = "-1"
    from_port   = 0
    to_port     = 0
    cidr_blocks = ["0.0.0.0/0"]
  }
}

data "aws_route53_zone" "geotool" {
  name         = var.domain_name
  provider = aws
  private_zone = false
}

resource "aws_route53_record" "geotool" {
  zone_id = data.aws_route53_zone.geotool.zone_id
  name    = var.domain_name
  type    = "A"
  ttl     = "300"
  records = [aws_instance.public-ec2.public_ip]
}

