#include <map>
#include <bit>
#include <vector>
#include <cstdio>
#include <cassert>

#include <riscv/simif.h>
#include <riscv/mmu.h>
#include <riscv/cfg.h>
#include <riscv/memtracer.h>

extern "C" typedef uint8_t* (*bus_addr_to_mem_callback_t)(void*, uint64_t);
extern "C" typedef void (*bus_mmio_callback_t)(void*, uint64_t, uint64_t, uint8_t*);

extern "C" struct tick_info_t {
    uint64_t time;
    uint8_t mtip;
    void* bus;
    bus_addr_to_mem_callback_t addr_to_mem;
    bus_mmio_callback_t mmio_load;
    bus_mmio_callback_t mmio_store;
};

extern "C" struct mem_log_t {
    uint64_t load_addr;
    uint64_t load_size;
    uint64_t store_addr;
    uint64_t store_val;
    uint64_t store_size;
};

extern "C" struct cosim_state_t {
    uint64_t pc;
    uint64_t prv;
    uint64_t regs[32];
    uint64_t sepc;
    uint64_t satp;
    uint64_t stval;
    uint64_t stvec;
    uint64_t scause;
    uint64_t sstatus;
    uint64_t mtval;
    uint64_t mcause;
    uint64_t mstatus;
    uint64_t mcountinhibit;
    uint64_t load_reservation;
};

class cosim_bridge_t : public simif_t {
public:
    cfg_t cfg;
    std::map<size_t, processor_t*> harts;

    tick_info_t* info;

    char* addr_to_mem(reg_t paddr) {
        return (char*)info->addr_to_mem(info->bus, paddr);
    }

    bool mmio_load(reg_t paddr, size_t len, uint8_t* bytes) {
        info->mmio_load(info->bus, paddr, len, bytes);
        return true;
    }

    bool mmio_store(reg_t paddr, size_t len, const uint8_t* bytes) {
        info->mmio_store(info->bus, paddr, len, (uint8_t*)bytes);
        return true;
    }

    void proc_reset(unsigned id) {}

    const cfg_t& get_cfg() const {
        return cfg;
    }
    const std::map<size_t, processor_t*>& get_harts() const {
        return harts;
    }
    const char* get_symbol(uint64_t paddr) {
        return nullptr;
    }
};

class cosim_mem_tracer_t : public memtracer_t {
public:
    uint64_t addr;

    bool interested_in_range(uint64_t begin, uint64_t end, access_type type) {
        return true;
    }

    void trace(uint64_t addr, size_t bytes, access_type type) {
        this->addr = addr;
    }

    void clean_invalidate(uint64_t addr, size_t bytes, bool clean, bool inval) {}
};

static processor_t* hart = nullptr;
static cosim_mem_tracer_t* tracer = nullptr;
static cosim_bridge_t* bridge = nullptr;

extern "C" void cosim_init(uint64_t boot_vector, uint64_t dt_addr) {
    bridge = new cosim_bridge_t;
    bridge->cfg.trigger_count = 0;
    bridge->cfg.misaligned = false;
    bridge->info = nullptr;

    tracer = new cosim_mem_tracer_t;    

    FILE* sink = fopen("/dev/null", "w");
    hart = new processor_t("RV64IMAC_Zicntr", "MSU", &bridge->cfg, bridge, 0, false, sink, std::cerr);
    hart->reset();
    hart->set_pmp_num(0);
    hart->enable_log_commits();
    hart->set_mmu_capability(69);
    hart->set_mmu_capability(1);
    hart->get_mmu()->register_memtracer(tracer);
    hart->get_state()->pc = boot_vector;
    *((uint64_t*)&hart->get_state()->XPR[11]) = dt_addr;
}

extern "C" mem_log_t cosim_tick(tick_info_t* tick_info) {
    bridge->info = tick_info;

    hart->get_state()->log_mem_write.clear();
    hart->get_state()->log_mem_read.clear();
    hart->get_state()->time->sync(tick_info->time);

    if(tick_info->mtip) {
        auto mip = hart->get_state()->mip->read();
        hart->get_state()->mip->backdoor_write_with_mask(1 << 7, mip | (1 << 7));
    }
    else {
        hart->get_state()->mip->backdoor_write_with_mask(1 << 7, 0);
    }
    
    uint64_t init_pc = hart->get_state()->pc;

    do {
        hart->step(1);
    } while(hart->get_state()->pc == init_pc);

    bridge->info = nullptr;

    mem_log_t log = {};

    if(hart->get_state()->log_mem_read.size() != 0) {
        assert(hart->get_state()->log_mem_read.size() == 1);

        auto [addr, val, size] = hart->get_state()->log_mem_read[0];
        log.load_addr = tracer->addr;
        log.load_size = size;
    }

    if(hart->get_state()->log_mem_write.size() != 0) {
        assert(hart->get_state()->log_mem_write.size() == 1);

        auto [addr, val, size] = hart->get_state()->log_mem_write[0];
        log.store_addr = tracer->addr;
        log.store_val = val;
        log.store_size = size;
    }

    return log;
}

extern "C" cosim_state_t cosim_state() {
    cosim_state_t state;
    state_t* st = hart->get_state();

    state.pc = st->pc;
    state.prv = st->prv;
    
    for(int i = 0; i < 32; i++) {
        state.regs[i] = st->XPR[i];
    }

    state.sepc = st->sepc->read();
    state.satp = st->satp->read();
    state.stval = st->stval->read();
    state.stvec = st->stvec->read();
    state.scause = st->scause->read();
    state.sstatus = st->sstatus->read();
    state.mtval = st->mtval->read();
    state.mcause = st->mcause->read();
    state.mstatus = st->mstatus->read();
    state.mcountinhibit = st->mcountinhibit->read();
    state.load_reservation = hart->get_mmu()->load_reservation_address;

    return state;
}
