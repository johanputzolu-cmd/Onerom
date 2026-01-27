# Contains mixed C64/VIC20 char roms
#
# Images:
# Set 0 - C64 char ROMs
# Set 1 - C64 char ROMs
# Set 2 - VIC-20 char ROMs (with C64 char ROM CS config)

ROM_CONFIGS = \
	set=0,bank=0,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/c64/characters.901225-01.bin,type=2332,cs1=0,cs2=1 \
	set=0,bank=1,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/c64/characters.325018-02.bin,type=2332,cs1=0,cs2=1 \
	set=0,bank=2,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/c64/characters.325056-03.bin,type=2332,cs1=0,cs2=1 \
	set=0,bank=3,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/c64/characters.901225-01-DK.bin,type=2332,cs1=0,cs2=1 \
	set=1,bank=0,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/c64/characters.901225-01.bin,type=2332,cs1=0,cs2=1 \
	set=1,bank=1,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/c64/characters.turkish.bin,type=2332,cs1=0,cs2=1 \
	set=1,bank=2,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/c64/croatian.zip,extract=c64_cro/chargen,type=2332,cs1=0,cs2=1 \
	set=1,bank=3,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/c64/characters.906143-02.bin,type=2332,cs1=0,cs2=1 \
	set=2,bank=0,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/vic20/characters.901460-03.bin,type=2332,cs1=0,cs2=1 \
	set=2,bank=1,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/vic20/characters.NecP22101-207.bin,type=2332,cs1=0,cs2=1 \
	set=2,bank=2,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/vic20/characters.DK_901460-03.bin,type=2332,cs1=0,cs2=1 \
	set=2,bank=3,file=http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/vic20/characters.901460-02.bin,type=2332,cs1=0,cs2=1