॰ संयोजकः — the compositor (C-010), doc 05 §9.
॰
॰ WHAT THIS PROGRAM IS
॰
॰ It composites three layers into an off-screen buffer and presents the result
॰ through virtio-gpu, double buffered, with the flip published only after the
॰ composite has completed. Two 1920x1080 B8G8R8X8 resources are created and
॰ backed by two separate 8294400-byte regions of guest memory; every frame is
॰ painted into whichever region is NOT currently on the scanout, and only then
॰ is SET_SCANOUT issued for it. That ordering is the whole no-tearing claim.
॰
॰ WHY THIS ORDERING IS THE CLAIM, IN QEMU'S TERMS
॰
॰ QEMU's virtio-gpu builds the console DisplaySurface directly over the pixel
॰ image of the resource named by SET_SCANOUT. TRANSFER_TO_HOST_2D writes into
॰ that image. So a driver that transfers into the resource it is scanning out
॰ is mutating the surface the host is showing, and a screendump taken during
॰ the composite is half one frame and half the next. That is exactly a tear,
॰ and it is observable from the host. The discipline here makes it impossible:
॰ the transfer always targets the resource that is not on the scanout.
॰
॰ HOW A READER CAN TELL A COMPOSITED FRAME APART FROM ANY OTHER FRAME
॰
॰ Every pixel of frame k carries k in its BLUE byte, whatever layer painted it.
॰ A frame that is one composite therefore has exactly ONE distinct blue value
॰ across all 2073600 pixels; a frame that is a mix of two composites has two.
॰ The layer identity lives in the RED byte instead — 0x11 background, 0xc4 the
॰ first rectangle, 0x2a the second — so the same picture also states the layer
॰ areas, and the second rectangle overlapping the first proves painter order:
॰
॰   background   1920x1080 minus the two rectangles   1173600 pixels, red 0x11
॰   layer 1      900x500 at (100,100), partly covered  300000 pixels, red 0xc4
॰   layer 2      1000x600 at (500,300), on top         600000 pixels, red 0x2a
॰
॰ If the layers were composited in the other order the two counts swap, so the
॰ ordering is falsifiable and not merely asserted.
॰
॰ THE CONTROL, WHICH RUNS FIRST AND ON PURPOSE
॰
॰ Before the tear-free loop this program deliberately tears once: it composites
॰ frame 1 into the FRONT buffer, presents it, then repaints the top half of that
॰ same still-scanned-out buffer with frame 2's blue and transfers it, holding the
॰ result on screen for about eight seconds and announcing the window on the
॰ serial line. During that window the screen carries TWO blue values. A check
॰ that cannot see the difference between that window and the loop is not
॰ measuring anything, so the check looks at both and requires both answers.
॰
॰ WHAT IS NOT CLAIMED
॰
॰ Nothing here demonstrates 60 frames per second. The geometry is the row's
॰ geometry — a real 1920x1080 surface, a real 8 MB transfer per frame — but the
॰ rate is whatever QEMU's interpreter manages, and the program prints the frame
॰ number and the elapsed `time` ticks after every present so the reader gets the
॰ measured rate rather than a claim about it.
॰
॰ REGISTER DIVISION (छापनम् clobbers क्षणिक२–क्षणिक५, अर्थ०, अर्थ७;
॰ छापवाक्यम् clobbers क्षणिक१ and क्षणिक६; आयतम् clobbers क्षणिक०–क्षणिक४)
॰   स्थिर०  device MMIO base        स्थिर६  framebuffer B
॰   स्थिर१  Q — the queue region    स्थिर७  length of the current command
॰   स्थिर२  used ring               स्थिर८  avail index
॰   स्थिर३  command buffer          स्थिर९  tag to print, or 0 for quiet
॰   स्थिर४  response buffer         स्थिर१० saved return address
॰   स्थिर५  framebuffer A           स्थिर११ resource id of the BACK buffer
॰
॰ SCRATCH, at स्थिर३ + 512 (Q + 0x2200), out of reach of the 64 bytes शुद्धिः
॰ clears and of the 64-byte response buffer at स्थिर३ + 256:
॰   +512  frame number      +552  संयोजनम् argument: destination framebuffer
॰   +520  start time        +560  संयोजनम् argument: frame index (blue byte)

॰ ============ 1. FINDING THE DEVICE ============
॰ Same scan as spec/virtio-gpu.sas: eight virtio-mmio slots 0x1000 apart from
॰ 0x10001000, magic "virt" and DeviceID 16. The address is found, not written
॰ down, so reordering -device on the command line cannot silently misaddress it.
उपरिभारः स्थिर०म् ०षोड्१०००१न ।
उपरिभारः स्थिर१म् ०षोड्१०००९न ।
उपरिभारः स्थिर२म् ०षोड्७४७२७न ।
योगः स्थिर२म् स्थिर२न ऋण०षोड्६८अन ।

अन्वेषणम्ॱॱ
समलङ्घनम् स्थिर०न स्थिर१त् नास्तिय् ।
आहारःॱअ३२ क्षणिक०म् स्थिर०त् ०न ।
विषमलङ्घनम् क्षणिक०न स्थिर२त् परमुपकरणम्य् ।
आहारःॱअ३२ क्षणिक०म् स्थिर०त् ८न ।
योगः क्षणिक१म् शून्यःन १६न ।
समलङ्घनम् क्षणिक०न क्षणिक१त् प्राप्तम्य् ।
परमुपकरणम्ॱॱ
उपरिभारः क्षणिक०म् १न ।
योगः स्थिर०म् स्थिर०न क्षणिक०न ।
लङ्घनम् शून्यःम् अन्वेषणम्य् ।

नास्तिॱॱ
स्थानसापेक्षयोगः क्षणिक६म् नास्तिवाक्यम्ॱउपरिन ।
योगः क्षणिक६म् क्षणिक६न नास्तिवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् छापवाक्यम्य् ।
लङ्घनम् शून्यःम् समाप्तिःय् ।

प्राप्तम्ॱॱ
॰ ============ 2. THE QUEUE REGION AND THE TWO FRAMEBUFFERS ============
॰ Q is computed with auipc and rounded up to a page: the used ring must land
॰ where QueueAlign puts it, and the link address is not known here.
स्थानसापेक्षयोगः स्थिर१म् क्षेत्रम्ॱउपरिन ।
योगः स्थिर१म् स्थिर१न क्षेत्रम्ॱअधःन ।
उपरिभारः क्षणिक०म् १न ।
योगः स्थिर१म् स्थिर१न क्षणिक०न ।
दक्षिणसरणम् स्थिर१म् स्थिर१न १२न ।
वामसरणम् स्थिर१म् स्थिर१न १२न ।
उपरिभारः क्षणिक०म् १न ।
योगः स्थिर२म् स्थिर१न क्षणिक०न ।
उपरिभारः क्षणिक०म् २न ।
योगः स्थिर३म् स्थिर१न क्षणिक०न ।
योगः स्थिर४म् स्थिर३न २५६न ।

॰ The framebuffers are 8294400 bytes each — too big to keep in ॱदत्त, where they
॰ would be 16 MB of zeros inside the ELF. They are in ॱरिक्त and every pixel of
॰ them is written before it is ever read or handed to the device, so nothing
॰ here depends on what the loader left there.
॰ 1920 x 1080 x 4 = 8294400 = 0x7e9000 exactly, so the stride between the two
॰ buffers is a single उपरिभारः with nothing to add.
स्थानसापेक्षयोगः स्थिर५म् चित्रक्षेत्रम्ॱउपरिन ।
योगः स्थिर५म् स्थिर५न चित्रक्षेत्रम्ॱअधःन ।
उपरिभारः क्षणिक०म् १न ।
योगः स्थिर५म् स्थिर५न क्षणिक०न ।
दक्षिणसरणम् स्थिर५म् स्थिर५न १२न ।
वामसरणम् स्थिर५म् स्थिर५न १२न ।
उपरिभारः क्षणिक०म् ०षोड्७उ९न ।
योगः स्थिर६म् स्थिर५न क्षणिक०न ।

योगः स्थिर८म् शून्यःन ०न ।
योगः स्थिर९म् शून्यःन ०न ।

स्थानसापेक्षयोगः क्षणिक६म् आधारवाक्यम्ॱउपरिन ।
योगः क्षणिक६म् क्षणिक६न आधारवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् छापवाक्यम्य् ।
योगः क्षणिक२म् स्थिर०न ०न ।
लङ्घनम् पुनःस्थानम्म् छापनम्य् ।

॰ ============ 3. DEVICE BRING-UP (virtio v1.2 §3.1.1, legacy transport) ============
निधानम्ॱअ३२ स्थिर०य् ११२न शून्यःन ।
योगः क्षणिक०म् शून्यःन १न ।
निधानम्ॱअ३२ स्थिर०य् ११२न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन ३न ।
निधानम्ॱअ३२ स्थिर०य् ११२न क्षणिक०न ।
निधानम्ॱअ३२ स्थिर०य् ३६न शून्यःन ।
निधानम्ॱअ३२ स्थिर०य् ३२न शून्यःन ।
उपरिभारः क्षणिक०म् १न ।
निधानम्ॱअ३२ स्थिर०य् ४०न क्षणिक०न ।
निधानम्ॱअ३२ स्थिर०य् ४८न शून्यःन ।
अचिह्नाहारःॱन३२ क्षणिक०म् स्थिर०त् ५२न ।
समलङ्घनम् क्षणिक०न शून्यःत् पंक्तिदोषःय् ।
योगः क्षणिक०म् शून्यःन ८न ।
निधानम्ॱअ३२ स्थिर०य् ५६न क्षणिक०न ।
उपरिभारः क्षणिक०म् १न ।
निधानम्ॱअ३२ स्थिर०य् ६०न क्षणिक०न ।
दक्षिणसरणम् क्षणिक०म् स्थिर१न १२न ।
निधानम्ॱअ३२ स्थिर०य् ६४न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन ७न ।
निधानम्ॱअ३२ स्थिर०य् ११२न क्षणिक०न ।

॰ ============ 4. THE AVAILABLE RING ============
निधानम्ॱअ१६ स्थिर१य् १२८न शून्यःन ।
निधानम्ॱअ१६ स्थिर१य् १३०न शून्यःन ।
योगः क्षणिक०म् स्थिर१न १३२न ।
योगः क्षणिक१म् शून्यःन ८न ।
वलयचक्रम्ॱॱ
निधानम्ॱअ१६ क्षणिक०य् ०न शून्यःन ।
योगः क्षणिक०म् क्षणिक०न २न ।
योगः क्षणिक१म् क्षणिक१न ऋण१न ।
विषमलङ्घनम् क्षणिक१न शून्यःत् वलयचक्रम्य् ।

॰ ============ 5. TWO RESOURCES, TWO BACKING STORES ============
॰ Resource 1 is backed by framebuffer A, resource 2 by framebuffer B. Nothing
॰ else in this program distinguishes them: which one is "front" is a property of
॰ SET_SCANOUT alone, and it changes every frame.
लङ्घनम् पुनःस्थानम्म् शुद्धिःय् ।
योगः क्षणिक०म् शून्यःन २५७न ।
निधानम्ॱअ३२ स्थिर३य् ०न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १न ।
निधानम्ॱअ३२ स्थिर३य् २४न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन २न ।
निधानम्ॱअ३२ स्थिर३य् २८न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १९२०न ।
निधानम्ॱअ३२ स्थिर३य् ३२न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १०८०न ।
निधानम्ॱअ३२ स्थिर३य् ३६न क्षणिक०न ।
योगः स्थिर७म् शून्यःन ४०न ।
योगः स्थिर८म् स्थिर८न १न ।
स्थानसापेक्षयोगः स्थिर९म् सृष्टिवाक्यम्ॱउपरिन ।
योगः स्थिर९म् स्थिर९न सृष्टिवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् कार्यम्य् ।

लङ्घनम् पुनःस्थानम्म् शुद्धिःय् ।
योगः क्षणिक०म् शून्यःन २६२न ।
निधानम्ॱअ३२ स्थिर३य् ०न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १न ।
निधानम्ॱअ३२ स्थिर३य् २४न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १न ।
निधानम्ॱअ३२ स्थिर३य् २८न क्षणिक०न ।
निधानम् स्थिर३य् ३२न स्थिर५न ।
उपरिभारः क्षणिक०म् ०षोड्७उ९न ।
निधानम्ॱअ३२ स्थिर३य् ४०न क्षणिक०न ।
योगः स्थिर७म् शून्यःन ४८न ।
योगः स्थिर८म् स्थिर८न १न ।
स्थानसापेक्षयोगः स्थिर९म् बन्धवाक्यम्ॱउपरिन ।
योगः स्थिर९म् स्थिर९न बन्धवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् कार्यम्य् ।

लङ्घनम् पुनःस्थानम्म् शुद्धिःय् ।
योगः क्षणिक०म् शून्यःन २५७न ।
निधानम्ॱअ३२ स्थिर३य् ०न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन २न ।
निधानम्ॱअ३२ स्थिर३य् २४न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन २न ।
निधानम्ॱअ३२ स्थिर३य् २८न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १९२०न ।
निधानम्ॱअ३२ स्थिर३य् ३२न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १०८०न ।
निधानम्ॱअ३२ स्थिर३य् ३६न क्षणिक०न ।
योगः स्थिर७म् शून्यःन ४०न ।
योगः स्थिर८म् स्थिर८न १न ।
स्थानसापेक्षयोगः स्थिर९म् सृष्टिवाक्यम्ॱउपरिन ।
योगः स्थिर९म् स्थिर९न सृष्टिवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् कार्यम्य् ।

लङ्घनम् पुनःस्थानम्म् शुद्धिःय् ।
योगः क्षणिक०म् शून्यःन २६२न ।
निधानम्ॱअ३२ स्थिर३य् ०न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन २न ।
निधानम्ॱअ३२ स्थिर३य् २४न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १न ।
निधानम्ॱअ३२ स्थिर३य् २८न क्षणिक०न ।
निधानम् स्थिर३य् ३२न स्थिर६न ।
उपरिभारः क्षणिक०म् ०षोड्७उ९न ।
निधानम्ॱअ३२ स्थिर३य् ४०न क्षणिक०न ।
योगः स्थिर७म् शून्यःन ४८न ।
योगः स्थिर८म् स्थिर८न १न ।
स्थानसापेक्षयोगः स्थिर९म् बन्धवाक्यम्ॱउपरिन ।
योगः स्थिर९म् स्थिर९न बन्धवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् कार्यम्य् ।

॰ ============ 6. FRAME 1, PRESENTED THE HONEST WAY ============
॰ Composite into A, transfer A into resource 1, put resource 1 on the scanout,
॰ flush. From here on resource 1 is the front buffer.
निधानम् स्थिर३य् ५५२न स्थिर५न ।
योगः क्षणिक०म् शून्यःन १न ।
निधानम् स्थिर३य् ५६०न क्षणिक०न ।
लङ्घनम् पुनःस्थानम्म् संयोजनम्य् ।

योगः स्थिर११म् शून्यःन १न ।
स्थानसापेक्षयोगः स्थिर९म् प्रेषणवाक्यम्ॱउपरिन ।
योगः स्थिर९म् स्थिर९न प्रेषणवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् प्रेषणम्य् ।
स्थानसापेक्षयोगः स्थिर९म् पटवाक्यम्ॱउपरिन ।
योगः स्थिर९म् स्थिर९न पटवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् दृश्यपटम्य् ।
स्थानसापेक्षयोगः स्थिर९म् प्रवाहवाक्यम्ॱउपरिन ।
योगः स्थिर९म् स्थिर९न प्रवाहवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् प्रवाहणम्य् ।

॰ ============ 7. THE CONTROL: ONE DELIBERATE TEAR ============
॰ Resource 1 is on the scanout. Its backing store is repainted — the top half
॰ only — with frame 2's blue, and transferred while it is still the scanout.
॰ This is the mistake the rest of the program is built to avoid, made on
॰ purpose, so that the check's detector can be shown to fire. The screen now
॰ carries blue 2 on rows 0..539 and blue 1 on rows 540..1079.
योगः अर्थ०म् स्थिर५न ०न ।
योगः अर्थ१म् शून्यःन ०न ।
योगः अर्थ२म् शून्यःन ०न ।
योगः अर्थ३म् शून्यःन १९२०न ।
योगः अर्थ४म् शून्यःन ५४०न ।
उपरिभारः अर्थ५म् ०षोड्११२न ।
योगः अर्थ५म् अर्थ५न ०षोड्२००न ।
योगः अर्थ५म् अर्थ५न २न ।
लङ्घनम् पुनःस्थानम्म् आयतम्य् ।

योगः स्थिर९म् शून्यःन ०न ।
लङ्घनम् पुनःस्थानम्म् प्रेषणम्य् ।
लङ्घनम् पुनःस्थानम्म् प्रवाहणम्य् ।

स्थानसापेक्षयोगः क्षणिक६म् स्फुटवाक्यम्ॱउपरिन ।
योगः क्षणिक६म् क्षणिक६न स्फुटवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् छापवाक्यम्य् ।

॰ Hold the torn picture for about 8.4 seconds — 20 << 22 ticks of the 10 MHz
॰ `time` counter — which is long enough for a check to notice the marker and
॰ ask QEMU for a screendump inside the window.
नियन्त्रकविकल्पः क्षणिक०म् ३०७३त् शून्यःन ।
योगः क्षणिक१म् शून्यःन २०न ।
वामसरणम् क्षणिक१म् क्षणिक१न २२न ।
योगः क्षणिक१म् क्षणिक०न क्षणिक१न ।
विलम्बचक्रम्ॱॱ
नियन्त्रकविकल्पः क्षणिक०म् ३०७३त् शून्यःन ।
अचिह्नन्यूनलङ्घनम् क्षणिक०न क्षणिक१त् विलम्बचक्रम्य् ।

स्थानसापेक्षयोगः क्षणिक६म् पिहितवाक्यम्ॱउपरिन ।
योगः क्षणिक६म् क्षणिक६न पिहितवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् छापवाक्यम्य् ।

॰ ============ 8. THE TEAR-FREE LOOP ============
॰ Back buffer starts as resource 2 (framebuffer B) because resource 1 is what
॰ the scanout is showing. Frame numbering resumes at 3 so that no frame in the
॰ loop can be confused with either of the control's two.
योगः स्थिर११म् शून्यःन २न ।
योगः क्षणिक०म् शून्यःन ३न ।
निधानम् स्थिर३य् ५१२न क्षणिक०न ।
नियन्त्रकविकल्पः क्षणिक०म् ३०७३त् शून्यःन ।
निधानम् स्थिर३य् ५२०न क्षणिक०न ।

स्थानसापेक्षयोगः क्षणिक६म् सज्जवाक्यम्ॱउपरिन ।
योगः क्षणिक६म् क्षणिक६न सज्जवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् छापवाक्यम्य् ।

चित्रचक्रम्ॱॱ
॰ The destination is the buffer belonging to the BACK resource, never the one
॰ on the scanout. स्थिर११ is the back resource id and alternates every frame.
योगः क्षणिक०म् शून्यःन १न ।
योगः क्षणिक१म् स्थिर६न ०न ।
विषमलङ्घनम् स्थिर११न क्षणिक०त् बफरनिश्चयःय् ।
योगः क्षणिक१म् स्थिर५न ०न ।
बफरनिश्चयःॱॱ
निधानम् स्थिर३य् ५५२न क्षणिक१न ।
आहारः क्षणिक०म् स्थिर३त् ५१२न ।
युक्तम् क्षणिक०म् क्षणिक०न ०षोड्ऊऊन ।
निधानम् स्थिर३य् ५६०न क्षणिक०न ।
लङ्घनम् पुनःस्थानम्म् संयोजनम्य् ।

॰ Quiet: the loop checks every response code and names its own failure, but it
॰ does not print two lines per command for hundreds of frames.
योगः स्थिर९म् शून्यःन ०न ।
लङ्घनम् पुनःस्थानम्म् प्रेषणम्य् ।
॰ THE FLIP. Everything above wrote into memory the host is not looking at.
॰ This one command is what publishes the finished composite, and it is a single
॰ device operation — there is no moment at which half of it has happened.
लङ्घनम् पुनःस्थानम्म् दृश्यपटम्य् ।
लङ्घनम् पुनःस्थानम्म् प्रवाहणम्य् ।

योगः क्षणिक०म् शून्यःन ३न ।
वियोगः स्थिर११म् क्षणिक०न स्थिर११न ।

आहारः क्षणिक०म् स्थिर३त् ५१२न ।
योगः क्षणिक०म् क्षणिक०न १न ।
निधानम् स्थिर३य् ५१२न क्षणिक०न ।

स्थानसापेक्षयोगः क्षणिक६म् पटलवाक्यम्ॱउपरिन ।
योगः क्षणिक६म् क्षणिक६न पटलवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् छापवाक्यम्य् ।
आहारः क्षणिक२म् स्थिर३त् ५१२न ।
लङ्घनम् पुनःस्थानम्म् छापनम्य् ।
नियन्त्रकविकल्पः क्षणिक२म् ३०७३त् शून्यःन ।
आहारः क्षणिक०म् स्थिर३त् ५२०न ।
वियोगः क्षणिक२म् क्षणिक२न क्षणिक०न ।
लङ्घनम् पुनःस्थानम्म् छापनम्य् ।
लङ्घनम् शून्यःम् चित्रचक्रम्य् ।

॰ ============ प्रेषणम्: TRANSFER_TO_HOST_2D for स्थिर११ ============
॰ These three take their resource id from स्थिर११ and their return address from
॰ the scratch word, because each of them calls कार्यम्, which needs पुनःस्थानम्.
प्रेषणम्ॱॱ
निधानम् स्थिर३य् ५६८न पुनःस्थानम्न ।
लङ्घनम् पुनःस्थानम्म् शुद्धिःय् ।
योगः क्षणिक०म् शून्यःन २६१न ।
निधानम्ॱअ३२ स्थिर३य् ०न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १९२०न ।
निधानम्ॱअ३२ स्थिर३य् ३२न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १०८०न ।
निधानम्ॱअ३२ स्थिर३य् ३६न क्षणिक०न ।
निधानम्ॱअ३२ स्थिर३य् ४८न स्थिर११न ।
योगः स्थिर७म् शून्यःन ५६न ।
योगः स्थिर८म् स्थिर८न १न ।
लङ्घनम् पुनःस्थानम्म् कार्यम्य् ।
आहारः पुनःस्थानम्म् स्थिर३त् ५६८न ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।

॰ ============ दृश्यपटम्: SET_SCANOUT for स्थिर११ ============
दृश्यपटम्ॱॱ
निधानम् स्थिर३य् ५६८न पुनःस्थानम्न ।
लङ्घनम् पुनःस्थानम्म् शुद्धिःय् ।
योगः क्षणिक०म् शून्यःन २५९न ।
निधानम्ॱअ३२ स्थिर३य् ०न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १९२०न ।
निधानम्ॱअ३२ स्थिर३य् ३२न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १०८०न ।
निधानम्ॱअ३२ स्थिर३य् ३६न क्षणिक०न ।
निधानम्ॱअ३२ स्थिर३य् ४४न स्थिर११न ।
योगः स्थिर७म् शून्यःन ४८न ।
योगः स्थिर८म् स्थिर८न १न ।
लङ्घनम् पुनःस्थानम्म् कार्यम्य् ।
आहारः पुनःस्थानम्म् स्थिर३त् ५६८न ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।

॰ ============ प्रवाहणम्: RESOURCE_FLUSH for स्थिर११ ============
प्रवाहणम्ॱॱ
निधानम् स्थिर३य् ५६८न पुनःस्थानम्न ।
लङ्घनम् पुनःस्थानम्म् शुद्धिःय् ।
योगः क्षणिक०म् शून्यःन २६०न ।
निधानम्ॱअ३२ स्थिर३य् ०न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १९२०न ।
निधानम्ॱअ३२ स्थिर३य् ३२न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन १०८०न ।
निधानम्ॱअ३२ स्थिर३य् ३६न क्षणिक०न ।
निधानम्ॱअ३२ स्थिर३य् ४०न स्थिर११न ।
योगः स्थिर७म् शून्यःन ४८न ।
योगः स्थिर८म् स्थिर८न १न ।
लङ्घनम् पुनःस्थानम्म् कार्यम्य् ।
आहारः पुनःस्थानम्म् स्थिर३त् ५६८न ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।

॰ ============ संयोजनम्: three layers into the destination ============
॰ Arguments in scratch: +552 the destination framebuffer, +560 the frame index.
॰ Layer order is the order of these three calls and nothing else; the check
॰ reads the areas back out of the picture, where a wrong order shows up.
संयोजनम्ॱॱ
योगः स्थिर१०म् पुनःस्थानम्न ०न ।

आहारः अर्थ०म् स्थिर३त् ५५२न ।
योगः अर्थ१म् शून्यःन ०न ।
योगः अर्थ२म् शून्यःन ०न ।
योगः अर्थ३म् शून्यःन १९२०न ।
योगः अर्थ४म् शून्यःन १०८०न ।
उपरिभारः अर्थ५म् ०षोड्११२न ।
योगः अर्थ५म् अर्थ५न ०षोड्२००न ।
आहारः क्षणिक५म् स्थिर३त् ५६०न ।
विकल्पः अर्थ५म् अर्थ५न क्षणिक५न ।
लङ्घनम् पुनःस्थानम्म् आयतम्य् ।

आहारः अर्थ०म् स्थिर३त् ५५२न ।
योगः अर्थ१म् शून्यःन १००न ।
योगः अर्थ२म् शून्यःन १००न ।
योगः अर्थ३म् शून्यःन ९००न ।
योगः अर्थ४म् शून्यःन ५००न ।
उपरिभारः अर्थ५म् ०षोड्इ४८न ।
योगः अर्थ५म् अर्थ५न ऋण०षोड्५००न ।
आहारः क्षणिक५म् स्थिर३त् ५६०न ।
विकल्पः अर्थ५म् अर्थ५न क्षणिक५न ।
लङ्घनम् पुनःस्थानम्म् आयतम्य् ।

आहारः अर्थ०म् स्थिर३त् ५५२न ।
योगः अर्थ१म् शून्यःन ५००न ।
योगः अर्थ२म् शून्यःन ३००न ।
योगः अर्थ३म् शून्यःन १०००न ।
योगः अर्थ४म् शून्यःन ६००न ।
उपरिभारः अर्थ५म् ०षोड्२अईन ।
आहारः क्षणिक५म् स्थिर३त् ५६०न ।
विकल्पः अर्थ५म् अर्थ५न क्षणिक५न ।
लङ्घनम् पुनःस्थानम्म् आयतम्य् ।

सापेक्षलङ्घनम् शून्यःम् स्थिर१०त् ०न ।

॰ ============ आयतम्: fill a rectangle ============
॰ अर्थ० base, अर्थ१ x, अर्थ२ y, अर्थ३ width, अर्थ४ height, अर्थ५ colour.
॰ Four pixels per iteration: every width used here (1920, 900, 1000) divides by
॰ four, and the unroll is worth it — the background layer alone is 2073600
॰ stores and this program does that ~11 times a second under an interpreter.
॰ Stride is 1920 x 4 = 7680, which is 15 << 9 and so needs no constant pool.
आयतम्ॱॱ
योगः क्षणिक०म् शून्यःन १५न ।
वामसरणम् क्षणिक०म् क्षणिक०न ९न ।
गुणनम् क्षणिक१म् अर्थ२न क्षणिक०न ।
योगः क्षणिक१म् अर्थ०न क्षणिक१न ।
वामसरणम् क्षणिक२म् अर्थ१न २न ।
योगः क्षणिक१म् क्षणिक१न क्षणिक२न ।
योगः क्षणिक३म् अर्थ४न ०न ।
समलङ्घनम् क्षणिक३न शून्यःत् आयतान्तःय् ।
आयतपङ्क्तिःॱॱ
योगः क्षणिक२म् क्षणिक१न ०न ।
दक्षिणसरणम् क्षणिक४म् अर्थ३न २न ।
समलङ्घनम् क्षणिक४न शून्यःत् पङ्क्त्यन्तःय् ।
आयतस्तम्भःॱॱ
निधानम्ॱअ३२ क्षणिक२य् ०न अर्थ५न ।
निधानम्ॱअ३२ क्षणिक२य् ४न अर्थ५न ।
निधानम्ॱअ३२ क्षणिक२य् ८न अर्थ५न ।
निधानम्ॱअ३२ क्षणिक२य् १२न अर्थ५न ।
योगः क्षणिक२म् क्षणिक२न १६न ।
योगः क्षणिक४म् क्षणिक४न ऋण१न ।
विषमलङ्घनम् क्षणिक४न शून्यःत् आयतस्तम्भःय् ।
पङ्क्त्यन्तःॱॱ
योगः क्षणिक१म् क्षणिक१न क्षणिक०न ।
योगः क्षणिक३म् क्षणिक३न ऋण१न ।
विषमलङ्घनम् क्षणिक३न शून्यःत् आयतपङ्क्तिःय् ।
आयतान्तःॱॱ
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।

॰ ============ शुद्धिः: clear the first 64 bytes of the command buffer ============
शुद्धिःॱॱ
निधानम् स्थिर३य् ०न शून्यःन ।
निधानम् स्थिर३य् ८न शून्यःन ।
निधानम् स्थिर३य् १६न शून्यःन ।
निधानम् स्थिर३य् २४न शून्यःन ।
निधानम् स्थिर३य् ३२न शून्यःन ।
निधानम् स्थिर३य् ४०न शून्यःन ।
निधानम् स्थिर३य् ४८न शून्यःन ।
निधानम् स्थिर३य् ५६न शून्यःन ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।

॰ ============ कार्यम्: submit one command, check the device's answer ============
॰ The response buffer is filled with 0xff first, so "the device never wrote
॰ anything" reads back as ffffffff and cannot pass for OK_NODATA. Every command
॰ in this program goes through here, including the hundreds the loop issues:
॰ the response is ALWAYS compared, and only the printing is optional.
कार्यम्ॱॱ
योगः स्थिर१०म् पुनःस्थानम्न ०न ।
योगः क्षणिक०म् शून्यःन ऋण१न ।
निधानम् स्थिर४य् ०न क्षणिक०न ।
निधानम् स्थिर४य् ८न क्षणिक०न ।
निधानम् स्थिर४य् १६न क्षणिक०न ।
निधानम् स्थिर१य् ०न स्थिर३न ।
निधानम्ॱअ३२ स्थिर१य् ८न स्थिर७न ।
योगः क्षणिक०म् शून्यःन १न ।
निधानम्ॱअ१६ स्थिर१य् १२न क्षणिक०न ।
निधानम्ॱअ१६ स्थिर१य् १४न क्षणिक०न ।
निधानम् स्थिर१य् १६न स्थिर४न ।
योगः क्षणिक०म् शून्यःन ६४न ।
निधानम्ॱअ३२ स्थिर१य् २४न क्षणिक०न ।
योगः क्षणिक०म् शून्यःन २न ।
निधानम्ॱअ१६ स्थिर१य् २८न क्षणिक०न ।
निधानम्ॱअ१६ स्थिर१य् ३०न शून्यःन ।
योगः क्षणिक०म् स्थिर८न ०न ।
लङ्घनम् पुनःस्थानम्म् अर्पणम्य् ।
॰ 0x1100 = VIRTIO_GPU_RESP_OK_NODATA. Compared before anything is printed, so a
॰ bad code cannot be lost among the loop's frame lines.
अचिह्नाहारःॱन३२ क्षणिक२म् स्थिर४त् ०न ।
योगः क्षणिक१म् शून्यःन १न ।
वामसरणम् क्षणिक१म् क्षणिक१न १२न ।
योगः क्षणिक१म् क्षणिक१न २५६न ।
विषमलङ्घनम् क्षणिक२न क्षणिक१त् आदेशदोषःय् ।
समलङ्घनम् स्थिर९न शून्यःत् मौनम्य् ।
योगः क्षणिक६म् स्थिर९न ०न ।
लङ्घनम् पुनःस्थानम्म् छापवाक्यम्य् ।
अचिह्नाहारःॱन३२ क्षणिक२म् स्थिर४त् ०न ।
लङ्घनम् पुनःस्थानम्म् छापनम्य् ।
मौनम्ॱॱ
सापेक्षलङ्घनम् शून्यःम् स्थिर१०त् ०न ।

॰ ============ अर्पणम्: hand over one request and wait for it ============
अर्पणम्ॱॱ
स्मृतिबन्धः पठनम्ऽलेखनम्त् पठनम्ऽलेखनम्य् ।
निधानम्ॱअ१६ स्थिर१य् १३०न क्षणिक०न ।
स्मृतिबन्धः पठनम्ऽलेखनम्त् आगमःऽनिर्गमःऽपठनम्ऽलेखनम्य् ।
निधानम्ॱअ३२ स्थिर०य् ८०न शून्यःन ।

प्रतीक्षाॱॱ
अचिह्नाहारःॱन१६ क्षणिक१म् स्थिर२त् २न ।
अचिह्नन्यूनलङ्घनम् क्षणिक१न क्षणिक०त् प्रतीक्षाय् ।
स्मृतिबन्धः आगमःऽनिर्गमःऽपठनम्ऽलेखनम्त् पठनम्ऽलेखनम्य् ।
अचिह्नाहारःॱन३२ क्षणिक१म् स्थिर०त् ९६न ।
निधानम्ॱअ३२ स्थिर०य् १००न क्षणिक१न ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।

॰ ============ छापवाक्यम्: the NUL-terminated string at क्षणिक६ ============
छापवाक्यम्ॱॱ
आहारःॱअ८ क्षणिक१म् क्षणिक६त् ०न ।
समलङ्घनम् क्षणिक१न शून्यःत् वाक्यान्तःय् ।
योगः अर्थ०म् क्षणिक१न ०न ।
योगः अर्थ७म् शून्यःन १न ।
आज्ञापनम् ।
योगः क्षणिक६म् क्षणिक६न १न ।
लङ्घनम् शून्यःम् छापवाक्यम्य् ।
वाक्यान्तःॱॱ
योगः अर्थ०म् शून्यःन १०न ।
योगः अर्थ७म् शून्यःन १न ।
आज्ञापनम् ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।

॰ ============ छापनम्: क्षणिक२ in hex, sixteen digits ============
छापनम्ॱॱ
योगः क्षणिक४म् शून्यःन ६०न ।
अङ्कचक्रम्ॱॱ
दक्षिणसरणम् क्षणिक५म् क्षणिक२न क्षणिक४न ।
युक्तम् क्षणिक५म् क्षणिक५न १५न ।
न्यूनम् क्षणिक३म् क्षणिक५न १०न ।
समलङ्घनम् क्षणिक३न शून्यःत् अक्षरम्य् ।
योगः अर्थ०म् क्षणिक५न ४८न ।
लङ्घनम् शून्यःम् मुद्रणम्य् ।
अक्षरम्ॱॱ
योगः अर्थ०म् क्षणिक५न ८७न ।
मुद्रणम्ॱॱ
योगः अर्थ७म् शून्यःन १न ।
आज्ञापनम् ।
योगः क्षणिक४म् क्षणिक४न ऋण४न ।
अन्यूनलङ्घनम् क्षणिक४न शून्यःत् अङ्कचक्रम्य् ।
योगः अर्थ०म् शून्यःन १०न ।
योगः अर्थ७म् शून्यःन १न ।
आज्ञापनम् ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।

॰ ============ NAMED FAILURES ============
॰ Each of these prints a tag of its own and then stands still. A compositor that
॰ dies quietly is indistinguishable from one that was never started, so every way
॰ this program can stop working has a name that reaches the serial line.
पंक्तिदोषःॱॱ
स्थानसापेक्षयोगः क्षणिक६म् पंक्तिवाक्यम्ॱउपरिन ।
योगः क्षणिक६म् क्षणिक६न पंक्तिवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् छापवाक्यम्य् ।
लङ्घनम् शून्यःम् चक्रःय् ।

आदेशदोषःॱॱ
स्थानसापेक्षयोगः क्षणिक६म् दोषवाक्यम्ॱउपरिन ।
योगः क्षणिक६म् क्षणिक६न दोषवाक्यम्ॱअधःन ।
लङ्घनम् पुनःस्थानम्म् छापवाक्यम्य् ।
अचिह्नाहारःॱन३२ क्षणिक२म् स्थिर४त् ०न ।
लङ्घनम् पुनःस्थानम्म् छापनम्य् ।
लङ्घनम् शून्यःम् चक्रःय् ।

चक्रःॱॱ
लङ्घनम् शून्यःम् चक्रःय् ।

समाप्तिःॱॱ
योगः अर्थ७म् शून्यःन ८न ।
आज्ञापनम् ।
लङ्घनम् शून्यःम् समाप्तिःय् ।

॥ कोष्ठकम् ॱदत्त ॥

नास्तिवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-NO-DEVICE ॥
॥ अष्टकाः ० ॥
पंक्तिवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-NO-QUEUE ॥
॥ अष्टकाः ० ॥
दोषवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-CMD-FAIL ॥
॥ अष्टकाः ० ॥
आधारवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-BASE ॥
॥ अष्टकाः ० ॥
सृष्टिवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-CREATE-2D ॥
॥ अष्टकाः ० ॥
बन्धवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-ATTACH-BACKING ॥
॥ अष्टकाः ० ॥
पटवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-SET-SCANOUT ॥
॥ अष्टकाः ० ॥
प्रेषणवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-TRANSFER ॥
॥ अष्टकाः ० ॥
प्रवाहवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-FLUSH ॥
॥ अष्टकाः ० ॥
स्फुटवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-TEAR-WINDOW ॥
॥ अष्टकाः ० ॥
पिहितवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-TEAR-CLOSED ॥
॥ अष्टकाः ० ॥
सज्जवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-READY ॥
॥ अष्टकाः ० ॥
पटलवाक्यम्ॱॱ
॥ आस्की COMPOSITOR-FRAME ॥
॥ अष्टकाः ० ॥

॰ The queue region: one page of alignment slack, then descriptors and the
॰ available ring, the used ring a QueueAlign further on, and the command page
॰ with the response buffer and the scratch words inside it. Four pages plus
॰ slack — 20480 bytes. It stays in ॱदत्त, as in spec/virtio-gpu.sas: the used
॰ ring's index must read zero before the first command, and in ॱरिक्त that
॰ would be a hope about the loader rather than a byte in the file.
क्षेत्रम्ॱॱ
॥ स्थानम् २०४८० ॥

॥ कोष्ठकम् ॱरिक्त ॥

॰ Two 1920x1080x4 framebuffers, 8294400 bytes each, plus a page of alignment
॰ slack: 16592896, rounded up to 16601088. In ॱदत्त this would put 16 MB of
॰ zeros into the ELF; it is safe in ॱरिक्त because every pixel is written by
॰ संयोजनम् before the device is ever pointed at it.
चित्रक्षेत्रम्ॱॱ
॥ स्थानम् १६६०१०८८ ॥
