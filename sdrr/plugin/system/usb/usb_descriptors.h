/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2019 Ha Thach (tinyusb.org)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

#ifndef USB_DESCRIPTORS_H_
#define USB_DESCRIPTORS_H_

#define ONE_ROM_USB_SYSTEM_PLUGIN_VID 0x2E8A
#define ONE_ROM_USB_SYSTEM_PLUGIN_PID 0x000F

enum
{
  VENDOR_REQUEST_MICROSOFT = 1
};

extern uint8_t const desc_ms_os_20[];

#define MS_OS_20_DESC_LEN  0xB2

#if CFG_TUSB_MCU == OPT_MCU_LPC175X_6X || CFG_TUSB_MCU == OPT_MCU_LPC177X_8X || CFG_TUSB_MCU == OPT_MCU_LPC40XX
  // LPC 17xx and 40xx endpoint type (bulk/interrupt/iso) are fixed by its number
  // 0 control, 1 In, 2 Bulk, 3 Iso, 4 In etc ...
  #define EPNUM_CDC_NOTIF  0x81
  #define EPNUM_CDC_OUT    0x02
  #define EPNUM_CDC_IN     0x82

  #define EPNUM_VENDOR_OUT 0x05
  #define EPNUM_VENDOR_IN  0x85

#elif CFG_TUSB_MCU == OPT_MCU_CXD56
  // CXD56 USB driver has fixed endpoint type (bulk/interrupt/iso) and direction (IN/OUT) by its number
  // 0 control (IN/OUT), 1 Bulk (IN), 2 Bulk (OUT), 3 In (IN), 4 Bulk (IN), 5 Bulk (OUT), 6 In (IN)
  #define EPNUM_CDC_NOTIF   0x83
  #define EPNUM_CDC_OUT     0x02
  #define EPNUM_CDC_IN      0x81

  #define EPNUM_VENDOR_OUT  0x05
  #define EPNUM_VENDOR_IN   0x84

#elif defined(TUD_ENDPOINT_ONE_DIRECTION_ONLY)
  // MCUs that don't support a same endpoint number with different direction IN and OUT defined in tusb_mcu.h
  //    e.g EP1 OUT & EP1 IN cannot exist together
  #define EPNUM_CDC_NOTIF   0x81
  #define EPNUM_CDC_OUT     0x02
  #define EPNUM_CDC_IN      0x83

  #define EPNUM_VENDOR_OUT  0x04
  #define EPNUM_VENDOR_IN   0x85

#else
  #define EPNUM_CDC_NOTIF   0x81
  #define EPNUM_CDC_OUT     0x02
  #define EPNUM_CDC_IN      0x82

  #define EPNUM_VENDOR_OUT  0x03
  #define EPNUM_VENDOR_IN   0x83
#endif

#endif /* USB_DESCRIPTORS_H_ */
