#pragma once
void pic_remap(void);
void pic_unmask_irq1(void);
void pic_send_eoi(unsigned char irq);