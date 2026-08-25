THEOS ?= $(HOME)/theos
ifeq ($(wildcard $(THEOS)/makefiles/common.mk),)
$(error 未找到 Theos，请设置 THEOS=/path/to/theos)
endif

TARGET := iphone:clang:16.5:15.0
ARCHS := arm64

include $(THEOS)/makefiles/common.mk

APPLICATION_NAME := A4NRemote

A4NRemote_FILES := main.m
A4NRemote_FRAMEWORKS := UIKit WebKit
A4NRemote_CFLAGS := -fobjc-arc -Wno-deprecated-declarations -Wall -Wextra -Werror
A4NRemote_RESOURCE_FILES := Resources/Info.plist Resources/AppIcon.png
A4NRemote_INSTALL_PATH := /Applications

include $(THEOS_MAKE_PATH)/application.mk

before-package::
	@rm -f "$(THEOS_PROJECT_DIR)/packages/A4NRemote_"*.ipa
