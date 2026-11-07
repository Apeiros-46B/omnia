mod server 'apps/server'
mod web 'apps/web'

build:
	just server build
	just web build

setup:
	just server setup
	just web setup
