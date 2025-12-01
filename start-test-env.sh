#!/bin/bash

# Quick start script for Docker testing environment

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}"
cat << "EOF"
╔═══════════════════════════════════════════╗
║   Rover RTC Network Testing Environment   ║
╚═══════════════════════════════════════════╝
EOF
echo -e "${NC}"

print_menu() {
    echo -e "\n${GREEN}What would you like to do?${NC}\n"
    echo "  1) Build and start containers"
    echo "  2) Interactive network switch test"
    echo "  3) Rapid switching test (5 cycles)"
    echo "  4) View logs"
    echo "  5) Stop containers"
    echo "  6) Clean up everything"
    echo "  7) Shell into peer container"
    echo "  8) Exit"
    echo -e "\n${YELLOW}Enter choice [1-8]:${NC} "
}

while true; do
    print_menu
    read choice
    
    case $choice in
        1)
            echo -e "\n${BLUE}Building and starting containers...${NC}"
            docker-compose up --build -d
            echo -e "\n${GREEN}✓ Containers started${NC}"
            echo -e "Server: ${BLUE}docker logs -f rover-server${NC}"
            echo -e "Peer:   ${BLUE}docker logs -f rover-peer${NC}"
            ;;
        2)
            if ! docker ps | grep -q rover-peer; then
                echo -e "\n${RED}Error: Containers not running. Start them first (option 1)${NC}"
            else
                ./test-network-switch.sh
            fi
            ;;
        3)
            if ! docker ps | grep -q rover-peer; then
                echo -e "\n${RED}Error: Containers not running. Start them first (option 1)${NC}"
            else
                ./test-rapid-switching.sh 5 15
            fi
            ;;
        4)
            echo -e "\n${GREEN}Choose logs to view:${NC}"
            echo "  1) Peer logs"
            echo "  2) Server logs"
            echo "  3) Both (side by side)"
            echo -n "Choice: "
            read log_choice
            case $log_choice in
                1) docker logs -f rover-peer ;;
                2) docker logs -f rover-server ;;
                3) docker-compose logs -f ;;
                *) echo -e "${RED}Invalid choice${NC}" ;;
            esac
            ;;
        5)
            echo -e "\n${BLUE}Stopping containers...${NC}"
            docker-compose stop
            echo -e "${GREEN}✓ Containers stopped${NC}"
            ;;
        6)
            echo -e "\n${YELLOW}This will remove all containers, networks, and volumes.${NC}"
            echo -n "Are you sure? (y/N): "
            read confirm
            if [[ $confirm == "y" || $confirm == "Y" ]]; then
                docker-compose down --volumes
                echo -e "${GREEN}✓ Cleanup complete${NC}"
            fi
            ;;
        7)
            if ! docker ps | grep -q rover-peer; then
                echo -e "\n${RED}Error: Peer container not running${NC}"
            else
                echo -e "\n${BLUE}Opening shell in peer container...${NC}"
                echo -e "${YELLOW}Type 'exit' to return to menu${NC}\n"
                docker exec -it rover-peer bash
            fi
            ;;
        8)
            echo -e "\n${GREEN}Goodbye!${NC}\n"
            exit 0
            ;;
        *)
            echo -e "\n${RED}Invalid choice. Please enter 1-8.${NC}"
            ;;
    esac
    
    echo -e "\n${YELLOW}Press ENTER to continue...${NC}"
    read
done
